//! Keeping PR worktrees under the disk cap (PLAN §5.7).

use std::path::Path;

use docent_git::{Git, WorktreesRoot};
use docent_store::{RecordedWorktree, Store};

use crate::command_error::CommandError;

/// Removes the least recently opened worktrees until the rest fit the cap.
/// The most recently opened one always stays. One that can't be measured or
/// removed is logged and left for next time.
pub async fn evict(store: &Store, git: &Git, worktrees: &Path) -> Result<(), CommandError> {
    let cap = store.worktree_cap().await?;
    let root = WorktreesRoot::new(worktrees);
    let mut measured = Vec::new();
    for worktree in store.worktrees().await? {
        let used = disk_use(&root, &worktree).await;
        measured.push((worktree, used));
    }
    let mut total: u64 = measured.iter().map(|(_, used)| used).sum();
    let Some((_newest, older)) = measured.split_last() else {
        return Ok(());
    };
    for (worktree, used) in older {
        if total <= cap {
            break;
        }
        match remove(store, git, &root, worktree).await {
            Ok(()) => total -= used,
            Err(err) => log(worktree, &err),
        }
    }
    Ok(())
}

/// The bytes `worktree` takes up, or zero if it can't be measured.
async fn disk_use(root: &WorktreesRoot, worktree: &RecordedWorktree) -> u64 {
    let dest = root.pr(worktree.repo_id, worktree.number);
    // Reads the whole tree, so it stays off the async threads.
    let used = tauri::async_runtime::spawn_blocking(move || dest.disk_use()).await;
    match used {
        Ok(Ok(used)) => used,
        Ok(Err(err)) => {
            log(worktree, &err.into());
            0
        }
        Err(err) => {
            eprintln!("Measuring a worktree stopped: {err}");
            0
        }
    }
}

async fn remove(
    store: &Store,
    git: &Git,
    root: &WorktreesRoot,
    worktree: &RecordedWorktree,
) -> Result<(), CommandError> {
    let dest = root.pr(worktree.repo_id, worktree.number);
    git.remove_pr_worktree(Path::new(&worktree.clone_path), &worktree.remote, &dest)
        .await?;
    store
        .forget_worktree(worktree.repo_id, worktree.number)
        .await?;
    Ok(())
}

fn log(worktree: &RecordedWorktree, err: &CommandError) {
    eprintln!(
        "Evicting PR {} of repo {}: {}",
        worktree.number, worktree.repo_id, err.message
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use docent_store::{NewRepo, RepoId};
    use std::path::PathBuf;
    use std::process::Command;

    fn sh_git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(args)
            .current_dir(dir)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// A store with a watched repo, and a clone to open its PRs from. Every
    /// PR's worktree takes up a little over a megabyte.
    struct Fixture {
        dir: tempfile::TempDir,
        store: Store,
        git: Git,
        repo: RepoId,
    }

    impl Fixture {
        async fn new() -> Self {
            let dir = tempfile::tempdir().expect("temp dir");
            sh_git(dir.path(), &["init", "--quiet", "-b", "main", "forge"]);
            let forge = dir.path().join("forge");
            std::fs::write(forge.join("big"), vec![1; 1_000_000]).expect("write");
            sh_git(&forge, &["add", "big"]);
            sh_git(&forge, &["commit", "--quiet", "-m", "big"]);
            sh_git(dir.path(), &["clone", "--quiet", "forge", "clone"]);
            let store = Store::open_in_memory().await.unwrap();
            let account = store.upsert_account("github.com", "octocat").await.unwrap();
            let repo = NewRepo {
                owner: "octocat".into(),
                name: "Hello-World".into(),
                default_branch: "main".into(),
            };
            let repo = store.watch_repo(account, &repo).await.unwrap().id;
            let git = Git::new(std::env::var_os("PATH").expect("tests run with a PATH"));
            Self {
                dir,
                store,
                git,
                repo,
            }
        }

        fn clone_path(&self) -> PathBuf {
            self.dir.path().join("clone")
        }

        fn worktrees(&self) -> PathBuf {
            self.dir.path().join("worktrees")
        }

        fn folder(&self, number: u64) -> PathBuf {
            let dest = WorktreesRoot::new(self.worktrees()).pr(self.repo, number);
            dest.path().to_path_buf()
        }

        async fn open(&self, number: u64) {
            let head = format!("refs/pull/{number}/head");
            sh_git(
                &self.dir.path().join("forge"),
                &["update-ref", &head, "main"],
            );
            let clone = self.clone_path();
            self.store
                .record_worktree(self.repo, number, clone.to_str().unwrap(), "origin")
                .await
                .expect("records");
            let dest = WorktreesRoot::new(self.worktrees()).pr(self.repo, number);
            self.git
                .open_pr_worktree(&clone, "origin", &dest)
                .await
                .expect("opens");
            self.store
                .touch_worktree(self.repo, number)
                .await
                .expect("touches");
        }

        async fn evict_to(&self, cap: u64) {
            self.store.set_worktree_cap(cap).await.unwrap();
            evict(&self.store, &self.git, &self.worktrees())
                .await
                .expect("evicts");
        }

        /// The PRs still recorded, least recently opened first.
        async fn recorded(&self) -> Vec<u64> {
            let worktrees = self.store.worktrees().await.unwrap();
            worktrees.iter().map(|w| w.number).collect()
        }
    }

    #[tokio::test]
    async fn the_least_recently_opened_go_first_until_the_rest_fit() {
        let f = Fixture::new().await;
        for number in [1, 2, 3, 4, 1] {
            f.open(number).await;
        }

        f.evict_to(2_500_000).await;

        assert_eq!(f.recorded().await, [4, 1]);
        for (number, stays) in [(1, true), (2, false), (3, false), (4, true)] {
            assert_eq!(f.folder(number).exists(), stays, "PR {number}");
        }
    }

    #[tokio::test]
    async fn nothing_is_removed_under_the_cap() {
        let f = Fixture::new().await;
        f.open(1).await;
        f.open(2).await;

        f.evict_to(10_000_000).await;

        assert_eq!(f.recorded().await, [1, 2]);
        assert!(f.folder(1).exists() && f.folder(2).exists());
    }

    #[tokio::test]
    async fn the_worktree_just_opened_stays_even_over_the_cap() {
        let f = Fixture::new().await;
        f.open(1).await;
        f.open(2).await;

        f.evict_to(0).await;

        assert_eq!(f.recorded().await, [2]);
        assert!(f.folder(2).exists());
    }

    /// The repo was bound to another clone, then unwatched. The worktree
    /// still belongs to the clone it was opened from.
    #[tokio::test]
    async fn a_worktree_of_a_rebound_and_unwatched_repo_is_removed_from_its_own_clone() {
        let f = Fixture::new().await;
        f.open(1).await;
        f.open(2).await;
        f.store.bind_repo(f.repo, "/code/other").await.unwrap();
        f.store.unwatch_repo(f.repo).await.unwrap();

        f.evict_to(0).await;

        assert_eq!(f.recorded().await, [2]);
        let listed = Command::new("git")
            .args(["worktree", "list"])
            .current_dir(f.clone_path())
            .output()
            .unwrap();
        let listed = String::from_utf8_lossy(&listed.stdout).to_string();
        assert!(!listed.contains("pr-1 "), "still registered: {listed}");
    }

    /// PR 1's folder was swapped for a symlink, so removing it is refused. It
    /// must not stop PR 2 going, and neither the symlink nor what it points
    /// to is touched.
    #[cfg(unix)]
    #[tokio::test]
    async fn one_that_cannot_be_removed_does_not_stop_the_rest() {
        let f = Fixture::new().await;
        for number in [1, 2, 3] {
            f.open(number).await;
        }
        let mine = f.dir.path().join("mine");
        std::fs::rename(f.folder(1), &mine).expect("move away");
        std::os::unix::fs::symlink(&mine, f.folder(1)).expect("symlink");

        f.evict_to(0).await;

        assert_eq!(f.recorded().await, [1, 3], "PR 1 is kept for next time");
        assert!(!f.folder(2).exists());
        assert!(mine.join("big").exists() && f.folder(1).is_symlink());
    }

    /// PR 1 can't be removed, so evicting the rest can't make up for it.
    #[cfg(unix)]
    #[tokio::test]
    async fn one_that_cannot_be_removed_does_not_cost_the_others_their_place() {
        use std::os::unix::fs::PermissionsExt;

        let f = Fixture::new().await;
        for number in [1, 2, 3] {
            f.open(number).await;
        }
        let locked = f.folder(1);
        let mode = std::fs::Permissions::from_mode;
        std::fs::set_permissions(&locked, mode(0o555)).expect("chmod");

        f.evict_to(2_500_000).await;

        std::fs::set_permissions(&locked, mode(0o755)).expect("chmod");
        assert_eq!(f.recorded().await, [1, 2, 3]);
    }
}

//! Removing the worktrees and pins of PRs that closed or merged (PLAN §5.7).

use std::path::Path;

use docent_forge::{Forge, PullRef};
use docent_git::{Git, WorktreesRoot};
use docent_store::{RecordedPull, Store};

use crate::command_error::CommandError;

/// The PRs on record that the forge says are closed or merged. A PR the
/// forge didn't answer for, on a host that failed or otherwise, isn't one.
pub async fn done_pulls(
    forge: &impl Forge,
    store: &Store,
) -> Result<Vec<RecordedPull>, CommandError> {
    let recorded = store.recorded_pulls().await?;
    if recorded.is_empty() {
        return Ok(Vec::new());
    }
    let refs: Vec<PullRef> = recorded.iter().map(pull_ref).collect();
    let answer = forge.pull_states(&refs).await;
    for failure in &answer.failures {
        eprintln!(
            "Could not ask {} which PRs closed: {}",
            failure.host, failure.error
        );
    }
    let done: Vec<&PullRef> = answer
        .states
        .iter()
        .filter(|(_, state)| state.is_done())
        .map(|(pull, _)| pull)
        .collect();
    Ok(recorded
        .into_iter()
        .filter(|pull| done.contains(&&pull_ref(pull)))
        .collect())
}

fn pull_ref(pull: &RecordedPull) -> PullRef {
    PullRef {
        host: pull.host.clone(),
        owner: pull.owner.clone(),
        repo: pull.name.clone(),
        number: pull.worktree.number,
    }
}

/// Removes the worktree and `refs/docent/*` pins of each of `done`, then
/// its record. One that can't be removed is logged and left for next time.
pub async fn remove(store: &Store, git: &Git, worktrees: &Path, done: &[RecordedPull]) {
    let root = WorktreesRoot::new(worktrees);
    for pull in done {
        if let Err(err) = remove_one(store, git, &root, pull).await {
            eprintln!(
                "Removing the worktree of closed PR {} of repo {}: {}",
                pull.worktree.number, pull.worktree.repo_id, err.message
            );
        }
    }
}

async fn remove_one(
    store: &Store,
    git: &Git,
    root: &WorktreesRoot,
    pull: &RecordedPull,
) -> Result<(), CommandError> {
    let worktree = &pull.worktree;
    let dest = root.pr(worktree.repo_id, worktree.number);
    git.remove_pr_worktree(Path::new(&worktree.clone_path), &worktree.remote, &dest)
        .await?;
    store
        .forget_worktree(worktree.repo_id, worktree.number)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use docent_forge::{ConnectError, FakeForge, HostFailure, PullState, PullStates};
    use docent_store::{NewRepo, RepoId};
    use std::path::PathBuf;
    use std::process::Command;

    fn sh_git(dir: &Path, args: &[&str]) -> String {
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
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// A store with a watched repo, and a clone to open its PRs from.
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
            sh_git(&forge, &["commit", "--quiet", "--allow-empty", "-m", "x"]);
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
            sh_git(&self.dir.path().join("forge"), &["update-ref", &head, "main"]);
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

        /// As eviction leaves it: the folder gone, the pins kept.
        async fn evict(&self, number: u64) {
            let dest = WorktreesRoot::new(self.worktrees()).pr(self.repo, number);
            self.git
                .evict_pr_worktree(&self.clone_path(), &dest)
                .await
                .expect("evicts");
            self.store.mark_evicted(self.repo, number).await.unwrap();
        }

        /// The PRs `forge` says are done, removed.
        async fn remove_done(&self, forge: &FakeForge) {
            let done = done_pulls(forge, &self.store).await.expect("asks");
            remove(&self.store, &self.git, &self.worktrees(), &done).await;
        }

        /// The PRs still on record.
        async fn recorded(&self) -> Vec<u64> {
            let pulls = self.store.recorded_pulls().await.unwrap();
            pulls.iter().map(|p| p.worktree.number).collect()
        }

        /// The PRs with pins left in the clone.
        fn pinned(&self) -> Vec<u64> {
            let refs = sh_git(
                &self.clone_path(),
                &["for-each-ref", "--format=%(refname)", "refs/docent/"],
            );
            let mut numbers: Vec<u64> = refs
                .lines()
                .filter_map(|name| name.split('/').nth(4)?.parse().ok())
                .collect();
            numbers.dedup();
            numbers
        }
    }

    fn pull(number: u64) -> PullRef {
        PullRef {
            host: "github.com".into(),
            owner: "octocat".into(),
            repo: "Hello-World".into(),
            number,
        }
    }

    fn forge(states: &[(u64, PullState)]) -> FakeForge {
        FakeForge::with_states(PullStates {
            states: states.iter().map(|&(n, s)| (pull(n), s)).collect(),
            failures: vec![],
        })
    }

    #[tokio::test]
    async fn closed_and_merged_prs_lose_their_worktrees_pins_and_records() {
        let f = Fixture::new().await;
        for number in [1, 2, 3] {
            f.open(number).await;
        }
        let forge = forge(&[
            (1, PullState::Closed),
            (2, PullState::Open),
            (3, PullState::Merged),
        ]);

        f.remove_done(&forge).await;

        assert_eq!(forge.states_asked(), [vec![pull(1), pull(2), pull(3)]]);
        assert_eq!(f.recorded().await, [2]);
        assert_eq!(f.pinned(), [2]);
        for (number, stays) in [(1, false), (2, true), (3, false)] {
            assert_eq!(f.folder(number).exists(), stays, "PR {number}");
        }
    }

    /// Found in #59: eviction keeps the pins, so they must still go.
    #[tokio::test]
    async fn an_evicted_prs_pins_go_when_it_closes() {
        let f = Fixture::new().await;
        f.open(1).await;
        f.evict(1).await;
        assert_eq!(f.pinned(), [1]);

        f.remove_done(&forge(&[(1, PullState::Merged)])).await;

        assert_eq!(f.pinned(), Vec::<u64>::new());
        assert_eq!(f.recorded().await, Vec::<u64>::new());
    }

    /// The denial: a host that fails says nothing about its PRs.
    #[tokio::test]
    async fn a_forge_error_removes_nothing() {
        let f = Fixture::new().await;
        f.open(1).await;
        let forge = FakeForge::with_states(PullStates {
            states: vec![(pull(1), PullState::Closed)],
            failures: vec![HostFailure {
                host: "github.com".into(),
                error: ConnectError::HttpStatus {
                    host: "github.com".into(),
                    status: 502,
                },
            }],
        });

        f.remove_done(&forge).await;

        assert_eq!(f.recorded().await, [1]);
        assert_eq!(f.pinned(), [1]);
        assert!(f.folder(1).exists());
    }

    /// The denial: a PR the forge doesn't answer for, say one deleted or
    /// hidden from the account, isn't taken as done.
    #[tokio::test]
    async fn a_pr_the_forge_does_not_know_is_kept() {
        let f = Fixture::new().await;
        f.open(1).await;

        f.remove_done(&forge(&[])).await;

        assert_eq!(f.recorded().await, [1]);
        assert!(f.folder(1).exists());
    }

    #[tokio::test]
    async fn with_nothing_on_record_the_forge_is_not_asked() {
        let f = Fixture::new().await;
        let forge = forge(&[]);

        f.remove_done(&forge).await;

        assert!(forge.states_asked().is_empty());
    }

    /// PR 1's folder was swapped for a symlink, so removing it is refused.
    /// PR 2 still goes, and neither the symlink nor what it points to is
    /// touched.
    #[cfg(unix)]
    #[tokio::test]
    async fn one_that_cannot_be_removed_is_kept_and_does_not_stop_the_rest() {
        let f = Fixture::new().await;
        f.open(1).await;
        f.open(2).await;
        let mine = f.dir.path().join("mine");
        std::fs::rename(f.folder(1), &mine).expect("move away");
        std::os::unix::fs::symlink(&mine, f.folder(1)).expect("symlink");

        f.remove_done(&forge(&[(1, PullState::Closed), (2, PullState::Closed)]))
            .await;

        assert_eq!(f.recorded().await, [1], "PR 1 is kept for next time");
        assert!(!f.folder(2).exists());
        assert!(mine.exists() && f.folder(1).is_symlink());
    }
}

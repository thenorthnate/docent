//! Opening and reading PR worktrees, and evicting them or removing those of
//! closed PRs, without one undoing the other.

use std::path::PathBuf;
use std::sync::Arc;

use docent_forge::Forge;
use docent_git::{ChangedFile, FileDiff, Git};
use docent_store::Store;
use tauri::async_runtime::JoinHandle;
use tokio::sync::RwLock;

use crate::changed_files::{self, FileBlobs, PullRange};
use crate::closed;
use crate::command_error::CommandError;
use crate::evict::evict;
use crate::worktree::{self, OpenedPr};

/// Read while a PR's worktree is being opened or its files listed or
/// diffed, written while worktrees are evicted or those of closed PRs
/// removed, so one is never removed while it is in use.
/// Whatever reads it must end with `evict_soon`.
pub type InUse = Arc<RwLock<()>>;

/// Docent's worktrees folder, and what working in it needs.
#[derive(Clone)]
pub struct Worktrees {
    pub in_use: InUse,
    pub store: Store,
    pub git: Git,
    pub root: PathBuf,
}

impl Worktrees {
    /// Opens a PR in its worktree, then enforces the disk cap in the
    /// background, so the PR shows without waiting for it. Evicts after a
    /// failed open too: an eviction may have been left to this one.
    pub async fn open(
        &self,
        host: &str,
        owner: &str,
        name: &str,
        number: u64,
    ) -> (Result<OpenedPr, CommandError>, JoinHandle<()>) {
        let opened = {
            let _opening = self.in_use.read().await;
            worktree::open(
                &self.store,
                &self.git,
                &self.root,
                host,
                owner,
                name,
                number,
            )
            .await
        };
        (opened, self.evict_soon())
    }

    /// Lists the files a PR changes from its open worktree, then enforces the
    /// disk cap in the background. An eviction may have been left to this
    /// listing, so it evicts after a failed one too.
    pub async fn changed_files(
        &self,
        range: &PullRange,
    ) -> (Result<Vec<ChangedFile>, CommandError>, JoinHandle<()>) {
        let files = {
            let _listing = self.in_use.read().await;
            changed_files::changed_files(&self.store, &self.git, &self.root, range).await
        };
        (files, self.evict_soon())
    }

    /// Diffs one of a PR's files in its open worktree, then enforces the disk
    /// cap in the background, as a listing does.
    pub async fn file_diff(
        &self,
        range: &PullRange,
        blobs: &FileBlobs,
    ) -> (Result<FileDiff, CommandError>, JoinHandle<()>) {
        let diff = {
            let _diffing = self.in_use.read().await;
            changed_files::file_diff(&self.store, &self.git, &self.root, range, blobs).await
        };
        (diff, self.evict_soon())
    }

    /// Removes the worktrees and pins of PRs the forge says are closed or
    /// merged. The forge is asked first, so nothing waits on it; the removal
    /// then waits for any open, listing or eviction to finish.
    pub async fn remove_done(&self, forge: &impl Forge) {
        let done = match closed::done_pulls(forge, &self.store).await {
            Ok(done) => done,
            Err(err) => {
                eprintln!("Could not check for closed PRs: {}", err.message);
                return;
            }
        };
        if done.is_empty() {
            return;
        }
        let _removing = self.in_use.write().await;
        // Read again under the lock, in case a PR was reopened from another
        // clone, or already removed, while the forge was asked.
        let recorded = match self.store.recorded_pulls().await {
            Ok(recorded) => recorded,
            Err(err) => {
                eprintln!("Could not check for closed PRs: {err}");
                return;
            }
        };
        let is_done = |pull: &docent_store::RecordedPull| {
            done.iter().any(|d| {
                d.worktree.repo_id == pull.worktree.repo_id
                    && d.worktree.number == pull.worktree.number
            })
        };
        let done: Vec<_> = recorded.into_iter().filter(is_done).collect();
        closed::remove(&self.store, &self.git, &self.root, &done).await;
    }

    /// Runs `evict_if_idle` in the background.
    fn evict_soon(&self) -> JoinHandle<()> {
        let worktrees = self.clone();
        tauri::async_runtime::spawn(async move { worktrees.evict_if_idle().await })
    }

    /// Enforces the disk cap, unless a PR is being opened, its files are being
    /// listed, or worktrees are already being evicted. Opening and listing end
    /// by evicting, so it is left to them.
    async fn evict_if_idle(&self) {
        let Ok(_evicting) = self.in_use.try_write() else {
            return;
        };
        if let Err(err) = evict(&self.store, &self.git, &self.root).await {
            eprintln!("Could not enforce the worktree disk cap: {}", err.message);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use docent_git::WorktreesRoot;
    use docent_store::NewRepo;
    use std::path::Path;
    use std::process::Command;
    use std::time::Duration;

    /// A watched repo bound to a clone that is its own forge, reached through
    /// a stand-in for ssh, so a test can hold a fetch open or fail it.
    struct Fixture {
        dir: tempfile::TempDir,
        worktrees: Worktrees,
        repo: i64,
    }

    impl Fixture {
        async fn new() -> Self {
            let dir = tempfile::tempdir().expect("temp dir");
            let base = dir.path().canonicalize().expect("canonicalize");
            let clone = base.join("clone");
            std::fs::create_dir(&clone).expect("clone folder");
            let store = Store::open_in_memory().await.unwrap();
            let account = store.upsert_account("github.com", "octocat").await.unwrap();
            let repo = NewRepo {
                owner: "octocat".into(),
                name: "Hello-World".into(),
                default_branch: "main".into(),
            };
            let repo = store.watch_repo(account, &repo).await.unwrap().id;
            store
                .bind_repo(repo, clone.to_str().unwrap())
                .await
                .unwrap();
            let worktrees = Worktrees {
                in_use: InUse::default(),
                store,
                git: Git::new(std::env::var_os("PATH").expect("tests run with a PATH")),
                root: base.join("worktrees"),
            };
            let f = Self {
                dir,
                worktrees,
                repo,
            };
            f.sh_git(&["init", "--quiet", "-b", "main"]);
            f.sh_git(&["commit", "--quiet", "--allow-empty", "-m", "x"]);
            let url = "git@github.com:octocat/Hello-World.git";
            f.sh_git(&["remote", "add", "origin", url]);
            f.sh_git(&["config", "ssh.variant", "simple"]);
            for number in [1, 2, 8] {
                f.sh_git(&["update-ref", &format!("refs/pull/{number}/head"), "main"]);
            }
            f.ssh_runs(&format!("exec git upload-pack {}", clone.display()));
            f
        }

        fn sh_git(&self, args: &[&str]) {
            let status = Command::new("git")
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .current_dir(self.dir.path().join("clone"))
                .status();
            assert!(status.expect("run git").success(), "git {args:?}");
        }

        /// What the next fetch runs in place of ssh.
        fn ssh_runs(&self, script: &str) {
            self.sh_git(&[
                "config",
                "core.sshCommand",
                &format!("sh -c '{script}' ssh"),
            ]);
        }

        fn marker(&self, name: &str) -> PathBuf {
            self.dir.path().join(name)
        }

        fn folder(&self, number: u64) -> PathBuf {
            let dest = WorktreesRoot::new(&self.worktrees.root).pr(self.repo, number);
            dest.path().to_path_buf()
        }

        /// Opens a PR and waits for the eviction after it.
        async fn open(&self, number: u64) -> Result<OpenedPr, CommandError> {
            let worktrees = self.worktrees.clone();
            open(worktrees, number).await
        }

        async fn cap(&self, bytes: u64) {
            self.worktrees.store.set_worktree_cap(bytes).await.unwrap();
        }
    }

    async fn open(worktrees: Worktrees, number: u64) -> Result<OpenedPr, CommandError> {
        let (opened, evicting) = worktrees
            .open("github.com", "octocat", "Hello-World", number)
            .await;
        evicting.await.expect("eviction finishes");
        opened
    }

    /// Lists PR 1's files from a base its clone doesn't have, so it is fetched,
    /// and waits for the eviction after it.
    async fn list(worktrees: Worktrees) -> Result<Vec<ChangedFile>, CommandError> {
        let range = PullRange {
            host: "github.com".into(),
            owner: "octocat".into(),
            repo: "Hello-World".into(),
            number: 1,
            base_sha: "1".repeat(40),
            head_sha: "2".repeat(40),
        };
        let (files, evicting) = worktrees.changed_files(&range).await;
        evicting.await.expect("eviction finishes");
        files
    }

    async fn wait_for(path: &Path) {
        for _ in 0..600 {
            if path.exists() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("{} never appeared", path.display());
    }

    #[tokio::test]
    async fn opening_a_pr_evicts_older_ones_over_the_cap() {
        let f = Fixture::new().await;
        f.cap(0).await;
        f.open(1).await.expect("opens");

        f.open(2).await.expect("opens");

        assert!(!f.folder(1).exists());
        assert!(f.folder(2).exists());
    }

    /// The denial: while PR 8 is being fetched, nothing is removed, even far
    /// over the cap. The skipped eviction then runs once that open is done,
    /// though it fails.
    #[tokio::test]
    async fn nothing_is_evicted_while_a_pr_is_being_opened_only_after() {
        let f = Fixture::new().await;
        f.open(1).await.expect("opens");
        f.open(2).await.expect("opens");
        f.cap(0).await;
        let (started, go) = (f.marker("started"), f.marker("go"));
        f.ssh_runs(&format!(
            "touch {}; while [ ! -e {} ]; do sleep 0.05; done; exit 1",
            started.display(),
            go.display()
        ));
        let opening = tokio::spawn(open(f.worktrees.clone(), 8));
        wait_for(&started).await;

        f.worktrees.evict_if_idle().await;

        assert!(f.folder(1).exists(), "evicted during an open");
        assert!(f.folder(2).exists(), "evicted during an open");
        std::fs::write(&go, "").expect("let the fetch go on");
        opening.await.unwrap().expect_err("the fetch fails");
        assert!(!f.folder(1).exists(), "the skipped eviction never ran");
        assert!(f.folder(2).exists(), "the newest stays");
    }

    /// The other half of the denial: an open that arrives during an eviction
    /// waits for it before touching anything.
    #[tokio::test]
    async fn an_open_waits_for_an_eviction_to_finish() {
        let f = Fixture::new().await;
        let evicting = f.worktrees.in_use.write().await;
        let opening = tokio::spawn(open(f.worktrees.clone(), 1));

        tokio::time::sleep(Duration::from_millis(500)).await;

        assert!(!opening.is_finished(), "opened during an eviction");
        assert!(f.worktrees.store.worktrees().await.unwrap().is_empty());
        drop(evicting);
        opening.await.unwrap().expect("opens");
        assert!(f.folder(1).exists());
    }

    /// The denial: while PR 1's files are being listed, nothing is removed,
    /// even far over the cap. The skipped eviction then runs once the listing
    /// is done, though it fails.
    #[tokio::test]
    async fn nothing_is_evicted_while_files_are_being_listed_only_after() {
        let f = Fixture::new().await;
        f.open(1).await.expect("opens");
        f.open(2).await.expect("opens");
        f.cap(0).await;
        let (started, go) = (f.marker("started"), f.marker("go"));
        f.ssh_runs(&format!(
            "touch {}; while [ ! -e {} ]; do sleep 0.05; done; exit 1",
            started.display(),
            go.display()
        ));
        let listing = tokio::spawn(list(f.worktrees.clone()));
        wait_for(&started).await;

        f.worktrees.evict_if_idle().await;

        assert!(f.folder(1).exists(), "evicted during a listing");
        std::fs::write(&go, "").expect("let the fetch go on");
        listing.await.unwrap().expect_err("the fetch fails");
        assert!(!f.folder(1).exists(), "the skipped eviction never ran");
        assert!(f.folder(2).exists(), "the newest stays");
    }

    /// The other half of the denial. The worktree is gone by the time the
    /// listing may start, so it says to reopen the PR.
    #[tokio::test]
    async fn a_listing_waits_for_an_eviction_and_says_if_its_worktree_went() {
        let f = Fixture::new().await;
        f.open(1).await.expect("opens");
        f.open(2).await.expect("opens");
        f.cap(0).await;
        let evicting = f.worktrees.in_use.write().await;
        let listing = tokio::spawn(list(f.worktrees.clone()));

        tokio::time::sleep(Duration::from_millis(500)).await;

        assert!(!listing.is_finished(), "listed during an eviction");
        evict(&f.worktrees.store, &f.worktrees.git, &f.worktrees.root)
            .await
            .expect("evicts");
        assert!(!f.folder(1).exists());
        drop(evicting);
        let err = listing.await.unwrap().expect_err("nothing to list");
        assert_eq!(err.kind, "worktree_missing");
        assert!(err.message.contains("Reopen the PR"), "{}", err.message);
    }

    /// The denial: a closed PR's worktree isn't removed while a PR is being
    /// opened, only once the open is done.
    #[tokio::test]
    async fn removing_closed_prs_waits_for_an_open_to_finish() {
        use docent_forge::{FakeForge, PullRef, PullState, PullStates};

        let f = Fixture::new().await;
        f.open(1).await.expect("opens");
        let closed = PullRef {
            host: "github.com".into(),
            owner: "octocat".into(),
            repo: "Hello-World".into(),
            number: 1,
        };
        let forge = FakeForge::with_states(PullStates {
            states: vec![(closed, PullState::Closed)],
            failures: vec![],
        });
        let opening = f.worktrees.in_use.read().await;
        let worktrees = f.worktrees.clone();
        let removing = tokio::spawn(async move { worktrees.remove_done(&forge).await });

        tokio::time::sleep(Duration::from_millis(500)).await;

        assert!(!removing.is_finished(), "removed during an open");
        assert!(f.folder(1).exists());
        drop(opening);
        removing.await.unwrap();
        assert!(!f.folder(1).exists());
    }

    /// PR 1's folder is a symlink, so evicting it is refused.
    #[tokio::test]
    async fn a_failed_eviction_does_not_fail_the_open() {
        let f = Fixture::new().await;
        f.open(1).await.expect("opens");
        f.cap(0).await;
        let mine = f.marker("mine");
        std::fs::rename(f.folder(1), &mine).expect("move away");
        std::os::unix::fs::symlink(&mine, f.folder(1)).expect("symlink");

        let opened = f.open(2).await.expect("opens");

        assert!(matches!(opened, OpenedPr::Ready { .. }), "{opened:?}");
        assert!(f.folder(2).exists());
        assert!(f.folder(1).is_symlink() && mine.exists());
    }
}

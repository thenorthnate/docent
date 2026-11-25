//! Opening PR worktrees and evicting them, without one undoing the other.

use std::path::PathBuf;
use std::sync::Arc;

use docent_git::Git;
use docent_store::Store;
use tauri::async_runtime::JoinHandle;
use tokio::sync::RwLock;

use crate::command_error::CommandError;
use crate::evict::evict;
use crate::worktree::{self, OpenedPr};

/// Read while a PR's worktree is being opened, written while worktrees are
/// evicted, so one is never removed as it is opened.
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
        let worktrees = self.clone();
        let evicting = tauri::async_runtime::spawn(async move { worktrees.evict_if_idle().await });
        (opened, evicting)
    }

    /// Enforces the disk cap, unless a PR is being opened or worktrees are
    /// already being evicted. Opening ends by evicting, so it is left to that.
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

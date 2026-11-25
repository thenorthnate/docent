//! Opening a PR's worktree (PLAN §4 "Worktree", §5.7).

use std::path::Path;

use docent_git::{Git, WorktreesRoot};
use docent_store::{Store, WatchedRepo};

use crate::bind::storable_path;
use crate::command_error::CommandError;

/// What opening a PR found, sent to the UI tagged by `status`.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OpenedPr {
    /// The worktree is checked out at the PR's head.
    Ready { path: String, head_sha: String },
    /// The repo has no local clone yet, so the UI offers Bind or Clone.
    Unbound { repo: WatchedRepo },
}

/// Opens PR `number` of `host:owner/name` in its worktree, at
/// `<worktrees>/<repo id>/pr-<number>`. The bound clone is checked again
/// first, since it may have changed since it was bound.
pub async fn open(
    store: &Store,
    git: &Git,
    worktrees: &Path,
    host: &str,
    owner: &str,
    name: &str,
    number: u64,
) -> Result<OpenedPr, CommandError> {
    let repo = store.watched_repo_named(host, owner, name).await?;
    let Some(local_path) = &repo.local_path else {
        return Ok(OpenedPr::Unbound { repo });
    };
    let clone = git
        .check_clone(Path::new(local_path), &repo.host, &repo.owner, &repo.name)
        .await?;
    // Before git starts, so even an open that fails halfway is known to the
    // disk cap.
    store
        .record_worktree(repo.id, number, storable_path(&clone)?, &clone.remote)
        .await?;
    let dest = WorktreesRoot::new(worktrees).pr(repo.id, number);
    let opened = git
        .open_pr_worktree(&clone.path, &clone.remote, &dest)
        .await?;
    Ok(OpenedPr::Ready {
        path: opened.path.display().to_string(),
        head_sha: opened.head_sha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use docent_store::NewRepo;

    async fn watched_hello_world(store: &Store) -> WatchedRepo {
        let account = store.upsert_account("github.com", "octocat").await.unwrap();
        let repo = NewRepo {
            owner: "octocat".into(),
            name: "Hello-World".into(),
            default_branch: "main".into(),
        };
        store.watch_repo(account, &repo).await.unwrap()
    }

    fn git() -> Git {
        Git::new(std::env::var_os("PATH").expect("tests run with a PATH"))
    }

    #[tokio::test]
    async fn an_unbound_repo_asks_for_a_clone_instead_of_failing() {
        let store = Store::open_in_memory().await.unwrap();
        let repo = watched_hello_world(&store).await;
        let data = tempfile::tempdir().unwrap();

        let opened = open(
            &store,
            &git(),
            data.path(),
            "github.com",
            "octocat",
            "Hello-World",
            7,
        )
        .await
        .expect("opens");

        assert_eq!(opened, OpenedPr::Unbound { repo });
        assert_eq!(
            std::fs::read_dir(data.path()).unwrap().count(),
            0,
            "nothing is created for an unbound repo"
        );
    }

    #[tokio::test]
    async fn a_repo_that_is_not_watched_is_refused() {
        let store = Store::open_in_memory().await.unwrap();
        let data = tempfile::tempdir().unwrap();

        let err = open(
            &store,
            &git(),
            data.path(),
            "github.com",
            "octocat",
            "Hello-World",
            7,
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind, "not_watched");
    }

    /// A clone that no longer matches the repo (say, its remote changed
    /// since binding) is reported, and no worktree is made from it.
    #[tokio::test]
    async fn a_bound_folder_that_is_no_longer_its_clone_is_refused() {
        let store = Store::open_in_memory().await.unwrap();
        let repo = watched_hello_world(&store).await;
        let folder = tempfile::tempdir().unwrap();
        store
            .bind_repo(repo.id, folder.path().to_str().unwrap())
            .await
            .unwrap();
        let data = tempfile::tempdir().unwrap();

        let err = open(
            &store,
            &git(),
            data.path(),
            "github.com",
            "octocat",
            "Hello-World",
            7,
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind, "not_a_repo");
        assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 0);
    }

    /// Only an open that works makes its PR the one in use. A failed one may
    /// still have left something on disk, so it is recorded all the same.
    #[tokio::test]
    async fn opening_records_the_worktree_and_marks_it_opened_if_it_works() {
        let store = Store::open_in_memory().await.unwrap();
        let repo = watched_hello_world(&store).await;
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().canonicalize().unwrap();
        let sh_git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .current_dir(&folder)
                .status();
            assert!(status.expect("run git").success(), "git {args:?}");
        };
        // The clone is its own forge, reached through a stand-in for ssh.
        let clone = folder.to_str().unwrap();
        let ssh = format!("sh -c 'exec git upload-pack {clone}' ssh");
        sh_git(&["init", "--quiet", "-b", "main"]);
        sh_git(&["commit", "--quiet", "--allow-empty", "-m", "x"]);
        sh_git(&[
            "remote",
            "add",
            "up",
            "git@github.com:octocat/Hello-World.git",
        ]);
        sh_git(&["config", "core.sshCommand", &ssh]);
        sh_git(&["config", "ssh.variant", "simple"]);
        sh_git(&["update-ref", "refs/pull/7/head", "main"]);
        sh_git(&["update-ref", "refs/pull/8/head", "main"]);
        store.bind_repo(repo.id, clone).await.unwrap();
        let data = tempfile::tempdir().unwrap();
        let git = git();
        let open_pr = |number| {
            let (host, owner, name) = ("github.com", "octocat", "Hello-World");
            open(&store, &git, data.path(), host, owner, name, number)
        };

        open_pr(7).await.expect("opens");
        open_pr(8).await.expect("opens");
        open_pr(7).await.expect("reopens");
        sh_git(&["config", "core.sshCommand", "false"]);
        open_pr(8).await.expect_err("the fetch fails");
        open_pr(9).await.expect_err("the fetch fails");

        let recorded = store.worktrees().await.unwrap();
        let order: Vec<_> = recorded.iter().map(|w| w.number).collect();
        assert_eq!(order, [9, 8, 7], "least recently opened first");
        assert_eq!(recorded[2].repo_id, repo.id);
        assert_eq!(recorded[2].clone_path, clone);
        assert_eq!(recorded[2].remote, "up");
    }
}

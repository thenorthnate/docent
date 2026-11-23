//! Opening a PR's worktree (PLAN §4 "Worktree", §5.7).

use std::path::Path;

use docent_git::Git;
use docent_store::{Store, WatchedRepo};

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
    let dest = worktrees
        .join(repo.id.to_string())
        .join(format!("pr-{number}"));
    let opened = git
        .open_pr_worktree(&clone.path, &clone.remote, number, worktrees, &dest)
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
}

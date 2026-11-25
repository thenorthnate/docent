//! Binding a watched repo to a local clone (PLAN §4, "Repo binding").

use std::path::Path;

use docent_git::{BoundClone, Git};
use docent_store::{Store, WatchedRepo};

use crate::command_error::CommandError;

/// Binds watched `repo` to the clone containing `folder`, once git confirms
/// one of its remotes is that repo. Nothing is stored otherwise.
pub async fn bind(
    store: &Store,
    git: &Git,
    repo: &WatchedRepo,
    folder: &Path,
) -> Result<WatchedRepo, CommandError> {
    let clone = git
        .check_clone(folder, &repo.host, &repo.owner, &repo.name)
        .await?;
    Ok(store.bind_repo(repo.id, storable_path(&clone)?).await?)
}

/// `clone`'s path as text, since that is how the store keeps it.
pub(crate) fn storable_path(clone: &BoundClone) -> Result<&str, CommandError> {
    clone.path.to_str().ok_or_else(|| CommandError {
        kind: "invalid_path",
        message: format!(
            "{} has characters Docent can't store. Move or rename the clone.",
            clone.path.display()
        ),
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

    #[tokio::test]
    async fn a_rejected_folder_leaves_the_repo_unbound() {
        let store = Store::open_in_memory().await.unwrap();
        let repo = watched_hello_world(&store).await;
        let folder = tempfile::tempdir().unwrap();
        let git = Git::new(std::env::var_os("PATH").expect("tests run with a PATH"));

        let err = bind(&store, &git, &repo, folder.path()).await.unwrap_err();

        assert_eq!(err.kind, "not_a_repo");
        assert_eq!(store.watched_repo(repo.id).await.unwrap().local_path, None);
    }
}

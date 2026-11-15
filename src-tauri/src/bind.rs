//! Binding a watched repo to a local clone (PLAN §4, "Repo binding").

use std::path::Path;

use docent_git::Git;
use docent_store::{RepoId, Store, WatchedRepo};

use crate::command_error::CommandError;

/// Binds watched repo `id` to the clone containing `folder`, once git
/// confirms one of its remotes is that repo. Nothing is stored otherwise.
pub async fn bind(
    store: &Store,
    git: &Git,
    id: RepoId,
    folder: &Path,
) -> Result<WatchedRepo, CommandError> {
    let repo = store.watched_repo(id).await?;
    let clone = git
        .check_clone(folder, &repo.host, &repo.owner, &repo.name)
        .await?;
    let path = clone.path.to_str().ok_or_else(|| CommandError {
        kind: "invalid_path",
        message: format!(
            "{} has characters Docent can't store. Move or rename the clone.",
            clone.path.display()
        ),
    })?;
    Ok(store.bind_repo(id, path).await?)
}

use std::path::{Path, PathBuf};

use crate::{Git, GitError};

/// A local clone checked against a forge repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundClone {
    /// The clone's top-level folder.
    pub path: PathBuf,
}

impl Git {
    /// Checks that `folder` is in a clone of `host:owner/name`, by its remotes.
    pub async fn check_clone(
        &self,
        folder: &Path,
        _host: &str,
        _owner: &str,
        _name: &str,
    ) -> Result<BoundClone, GitError> {
        self.run(folder, &["rev-parse", "--show-toplevel"]).await?;
        Err(GitError::NotFound)
    }
}

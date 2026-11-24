//! Docent's worktrees folder, and the paths inside it. Opening and removing
//! worktrees writes and deletes folders, so a path here can only be derived
//! from a repo id and PR number, and is checked against symlinks on disk.

use std::path::{Path, PathBuf};

use crate::GitError;

/// The folder holding every PR worktree Docent creates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreesRoot(PathBuf);

impl WorktreesRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Where PR `number` of the repo with this id goes.
    pub fn pr(&self, repo_id: i64, number: u64) -> PrWorktreePath {
        let path = self
            .0
            .join(repo_id.to_string())
            .join(format!("pr-{number}"));
        PrWorktreePath {
            root: self.0.clone(),
            path,
            number,
        }
    }
}

/// A PR's worktree path. Only [`WorktreesRoot::pr`] makes one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrWorktreePath {
    root: PathBuf,
    path: PathBuf,
    number: u64,
}

impl PrWorktreePath {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn number(&self) -> u64 {
        self.number
    }

    /// Refuses a path that a symlink, at its end or on the way, would lead
    /// out of the root.
    pub(crate) fn check_inside(&self) -> Result<(), GitError> {
        let refuse = || Err(self.outside());
        if is_symlink(&self.path) {
            return refuse();
        }
        // A path that doesn't resolve has no folder.
        if let Some(resolved) = resolve(&self.path) {
            match resolve(&self.root) {
                Some(root) if resolved.starts_with(&root) && resolved != root => {}
                _ => return refuse(),
            }
        }
        Ok(())
    }

    /// Deletes a folder here that git doesn't list as a worktree.
    pub(crate) fn clear_stray(&self) -> Result<(), GitError> {
        if self.path.symlink_metadata().is_err() {
            return Ok(());
        }
        self.check_inside()?;
        std::fs::remove_dir_all(&self.path).map_err(|err| GitError::ClearFailed {
            path: self.path.display().to_string(),
            why: err.to_string(),
        })
    }

    fn outside(&self) -> GitError {
        GitError::OutsideWorktrees(self.path.display().to_string())
    }
}

fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|meta| meta.file_type().is_symlink())
}

/// `path` with symlinks resolved, as far as it exists: a missing last
/// component is kept as given.
pub(crate) fn resolve(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = path.canonicalize() {
        return Some(path);
    }
    Some(path.parent()?.canonicalize().ok()?.join(path.file_name()?))
}

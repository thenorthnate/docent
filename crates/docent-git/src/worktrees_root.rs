//! Docent's worktrees folder, and the paths inside it. Opening and removing
//! worktrees writes and deletes folders, so a path here can only be derived
//! from a repo id and PR number, and is checked against symlinks on disk.

use std::os::fd::{AsFd, OwnedFd};
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, Dir, Mode, OFlags, openat, unlinkat};
use rustix::io::Errno;
use rustix::path::Arg;

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

    /// Refuses a path with a symlink below the root, which could lead to the
    /// user's files or to another repo's worktrees. The root itself may be
    /// reached through one.
    pub(crate) fn check_inside(&self) -> Result<(), GitError> {
        let repo_dir = self.path.parent().unwrap_or(&self.root);
        if is_symlink(repo_dir) || is_symlink(&self.path) {
            return Err(self.outside());
        }
        Ok(())
    }

    /// Deletes anything here that git doesn't list as a worktree. Works down
    /// from a handle on the root, never following a symlink below it, so a
    /// folder swapped for one mid-way can't lead the delete elsewhere.
    pub(crate) fn clear_stray(&self) -> Result<(), GitError> {
        self.check_inside()?;
        match self.clear() {
            Ok(()) | Err(Errno::NOENT) => Ok(()),
            Err(Errno::LOOP | Errno::NOTDIR) => Err(self.outside()),
            Err(err) => Err(GitError::ClearFailed {
                path: self.path.display().to_string(),
                why: err.to_string(),
            }),
        }
    }

    fn clear(&self) -> Result<(), Errno> {
        let (Some(repo), Some(leaf)) = (
            self.path.parent().and_then(Path::file_name),
            self.path.file_name(),
        ) else {
            return Err(Errno::INVAL);
        };
        let root = rustix::fs::open(
            &self.root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let repo = open_dir_at(&root, repo)?;
        remove_at(&repo, leaf)
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

/// Opens folder `name` in `parent`, failing if it is a symlink.
fn open_dir_at(parent: impl AsFd, name: impl Arg) -> Result<OwnedFd, Errno> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    openat(parent, name, flags, Mode::empty())
}

/// Deletes `name` in `parent` and everything below it, without following
/// symlinks.
fn remove_at(parent: &OwnedFd, name: impl Arg + Copy) -> Result<(), Errno> {
    match unlinkat(parent, name, AtFlags::empty()) {
        // It's a folder: Linux says ISDIR, macOS PERM.
        Err(Errno::ISDIR | Errno::PERM) => {}
        done => return done,
    }
    let dir = open_dir_at(parent, name)?;
    for entry in Dir::read_from(&dir)?.collect::<Result<Vec<_>, _>>()? {
        let name = entry.file_name();
        if name != c"." && name != c".." {
            remove_at(&dir, name)?;
        }
    }
    unlinkat(parent, name, AtFlags::REMOVEDIR)
}

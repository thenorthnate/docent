//! Docent's worktrees folder, and the paths inside it. Opening and removing
//! worktrees writes and deletes folders, so a path here can only be derived
//! from a repo id and PR number, and is checked against symlinks on disk.

use std::ffi::{CStr, CString, OsStr};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, Stat, openat, renameat, statat, unlinkat};
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
        let (repo, leaf) = (repo_id.to_string(), format!("pr-{number}"));
        PrWorktreePath {
            path: self.0.join(&repo).join(&leaf),
            root: self.0.clone(),
            repo,
            leaf,
            number,
        }
    }
}

/// A PR's worktree path. Only [`WorktreesRoot::pr`] makes one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrWorktreePath {
    root: PathBuf,
    repo: String,
    leaf: String,
    /// `root/repo/leaf`.
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
        if is_symlink(&self.root.join(&self.repo)) || is_symlink(&self.path) {
            return Err(self.outside());
        }
        Ok(())
    }

    /// Deletes anything here that git doesn't list as a worktree. Works down
    /// from a handle on the root, never following a symlink below it, so a
    /// folder swapped for one mid-way can't lead the delete elsewhere.
    pub(crate) fn clear_stray(&self) -> Result<(), GitError> {
        self.check_inside()?;
        let cleared = match self.open_repo_dir() {
            Ok(repo) => remove_at(&repo, self.leaf.as_str()),
            Err(Errno::LOOP | Errno::NOTDIR) => return Err(self.outside()),
            Err(err) => Err(err),
        };
        gone_is_ok(cleared).map_err(|err| GitError::ClearFailed {
            path: self.path.display().to_string(),
            why: err.to_string(),
        })
    }

    /// The bytes this worktree takes up on disk. Zero if it doesn't exist.
    /// Follows no symlink it finds; a folder swapped for one mid-way can make
    /// the number wrong, but nothing is changed. Reads the whole tree, so call
    /// it off the async threads.
    pub fn disk_use(&self) -> Result<u64, GitError> {
        self.check_inside()?;
        let measured = self
            .open_repo_dir()
            .and_then(|repo| disk_use_at(&repo, self.leaf.as_str()));
        match measured {
            Err(Errno::NOENT) => Ok(0),
            measured => measured.map_err(|err| GitError::MeasureFailed {
                path: self.path.display().to_string(),
                why: err.to_string(),
            }),
        }
    }

    fn open_repo_dir(&self) -> Result<OwnedFd, Errno> {
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
        let root = rustix::fs::open(&self.root, flags, Mode::empty())?;
        open_dir_at(&root, self.repo.as_str())
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
///
/// A PR can nest folders deeper than the open-file limit, so this doesn't
/// hold one handle per level: each pass moves the folders two levels down up
/// to the top, then deletes the emptied level.
fn remove_at(parent: &OwnedFd, name: &str) -> Result<(), Errno> {
    if !is_dir_at(parent, name)? {
        return unlinkat(parent, name, AtFlags::empty());
    }
    let top = open_dir_at(parent, name)?;
    let mut moved = 0u64;
    // One pass per level of nesting. The bound only stops a delete that
    // something else keeps refilling.
    for _ in 0..MAX_PASSES {
        let entries = entries(&top)?;
        if entries.is_empty() {
            break;
        }
        for entry in &entries {
            gone_is_ok(remove_level(&top, entry, &mut moved))?;
        }
    }
    unlinkat(parent, name, AtFlags::REMOVEDIR)
}

const MAX_PASSES: usize = 100_000;

/// Deletes `name` in `top`. If it's a folder, its own folders are moved into
/// `top` first, for the next pass.
fn remove_level(top: &OwnedFd, name: &CStr, moved: &mut u64) -> Result<(), Errno> {
    if !is_dir_at(top, name)? {
        return unlinkat(top, name, AtFlags::empty());
    }
    let dir = open_dir_at(top, name)?;
    for entry in &entries(&dir)? {
        let done = is_dir_at(&dir, entry).and_then(|is_dir| match is_dir {
            true => move_up(&dir, entry, top, moved),
            false => unlinkat(&dir, entry, AtFlags::empty()),
        });
        gone_is_ok(done)?;
    }
    match unlinkat(top, name, AtFlags::REMOVEDIR) {
        // Something was added meanwhile: the next pass takes it.
        Err(Errno::NOTEMPTY | Errno::EXIST) => Ok(()),
        done => done,
    }
}

/// Moves `name` in `dir` into `top`, under a name not taken there.
fn move_up(dir: &OwnedFd, name: &CStr, top: &OwnedFd, moved: &mut u64) -> Result<(), Errno> {
    loop {
        *moved += 1;
        let new_name = format!(".docent-clearing-{moved}");
        match renameat(dir, name, top, new_name.as_str()) {
            Err(Errno::NOTEMPTY | Errno::EXIST | Errno::NOTDIR) => {}
            done => return done,
        }
    }
}

/// Bytes in use by `name` in `parent` and below. Opens each folder from the
/// top rather than holding one handle per level.
fn disk_use_at(parent: &OwnedFd, name: &str) -> Result<u64, Errno> {
    let mut used = allocated(&statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)?);
    if !is_dir_at(parent, name)? {
        return Ok(used);
    }
    let top = open_dir_at(parent, name)?;
    let mut folders = vec![PathBuf::from(".")];
    while let Some(folder) = folders.pop() {
        let dir = match open_dir_at(&top, &folder) {
            // Deleted or replaced meanwhile.
            Err(Errno::NOENT | Errno::NOTDIR | Errno::LOOP) => continue,
            dir => dir?,
        };
        for entry in entries(&dir)? {
            let stat = match statat(&dir, &entry, AtFlags::SYMLINK_NOFOLLOW) {
                Err(Errno::NOENT) => continue,
                stat => stat?,
            };
            used += allocated(&stat);
            if FileType::from_raw_mode(stat.st_mode).is_dir() {
                folders.push(folder.join(OsStr::from_bytes(entry.as_bytes())));
            }
        }
    }
    Ok(used)
}

/// `st_blocks` is in 512-byte units whatever the file system's block size.
fn allocated(stat: &Stat) -> u64 {
    u64::try_from(stat.st_blocks).unwrap_or(0) * 512
}

fn is_dir_at(parent: &OwnedFd, name: impl Arg) -> Result<bool, Errno> {
    let stat = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
    Ok(FileType::from_raw_mode(stat.st_mode).is_dir())
}

/// The names in `dir`, without `.` and `..`.
fn entries(dir: &OwnedFd) -> Result<Vec<CString>, Errno> {
    let mut names = Vec::new();
    for entry in Dir::read_from(dir)? {
        let entry = entry?;
        if entry.file_name() != c"." && entry.file_name() != c".." {
            names.push(entry.file_name().to_owned());
        }
    }
    Ok(names)
}

/// Deleting what is already gone is fine.
fn gone_is_ok(result: Result<(), Errno>) -> Result<(), Errno> {
    match result {
        Err(Errno::NOENT) => Ok(()),
        result => result,
    }
}

//! One changed file's contents and git's hunks, for the diff viewer
//! (PLAN §5.4).

use std::path::Path;

use crate::changed_files::is_full_sha;
use crate::runner::NETWORK_TIMEOUT;
use crate::{Git, GitError, PrWorktreePath};

/// The most either side of a file may hold for its diff to be shown.
pub const MAX_DIFF_BYTES: u64 = 4 * 1024 * 1024;

/// How far into a file git looks for a NUL byte to call it binary.
const BINARY_SNIFF: usize = 8000;

/// What the viewer shows for one changed file. Contents come from the PR, so
/// they are untrusted: the UI shows them as text only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileDiff {
    /// Both sides as text (bytes that aren't UTF-8 replaced, which never adds
    /// or removes a line), and git's `-U0` hunks between them.
    Text {
        old: String,
        new: String,
        hunks: Vec<Hunk>,
    },
    Binary,
    /// A side is over [`MAX_DIFF_BYTES`]; `bytes` is the larger side's size.
    TooLarge {
        bytes: u64,
    },
}

/// A hunk as git's `@@ -old_start,old_lines +new_start,new_lines @@` header
/// gives it. Lines count from 1. A side with no lines starts at the line
/// before the change, as in git.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Hunk {
    pub old_start: u64,
    pub old_lines: u64,
    pub new_start: u64,
    pub new_lines: u64,
}

impl Git {
    /// The diff between blobs `old_blob` and `new_blob` (`None` for a side the
    /// file doesn't exist on), as listed by [`Git::changed_files`], read in
    /// `clone`'s open worktree at `dest`.
    pub async fn file_diff(
        &self,
        clone: &Path,
        dest: &PrWorktreePath,
        old_blob: Option<&str>,
        new_blob: Option<&str>,
    ) -> Result<FileDiff, GitError> {
        for sha in [old_blob, new_blob].into_iter().flatten() {
            if !is_full_sha(sha) {
                return Err(GitError::InvalidSha(sha.to_string()));
            }
        }
        let worktree = self.checked_worktree(clone, dest).await?;
        let mut largest = 0;
        for sha in [old_blob, new_blob].into_iter().flatten() {
            let size = self.git(worktree, &["cat-file", "-s", sha]).await?;
            let size = size.trim().parse().map_err(|_| unexpected("cat-file -s"))?;
            largest = largest.max(size);
        }
        if largest > MAX_DIFF_BYTES {
            return Ok(FileDiff::TooLarge { bytes: largest });
        }
        let (old, new) = (
            self.contents(worktree, old_blob).await?,
            self.contents(worktree, new_blob).await?,
        );
        // Git's own rule when no attributes apply. A PR's `.gitattributes`
        // can't apply here, so it can't hide a text file's changes.
        if [&old, &new]
            .iter()
            .any(|bytes| bytes[..bytes.len().min(BINARY_SNIFF)].contains(&0))
        {
            return Ok(FileDiff::Binary);
        }
        let hunks = match (old_blob, new_blob) {
            (Some(old_blob), Some(new_blob)) if old_blob != new_blob => {
                self.hunks(worktree, old_blob, new_blob).await?
            }
            (None, Some(_)) => whole(&new, |lines| Hunk {
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: lines,
            }),
            (Some(_), None) => whole(&old, |lines| Hunk {
                old_start: 1,
                old_lines: lines,
                new_start: 0,
                new_lines: 0,
            }),
            _ => Vec::new(),
        };
        Ok(FileDiff::Text {
            old: String::from_utf8_lossy(&old).into_owned(),
            new: String::from_utf8_lossy(&new).into_owned(),
            hunks,
        })
    }

    /// `git <args>`, hardened, given as long as a fetch: a partial clone
    /// fetches the objects it lacks.
    async fn git(&self, worktree: &Path, args: &[&str]) -> Result<String, GitError> {
        self.run_hardened(worktree, args, NETWORK_TIMEOUT).await
    }

    async fn contents(&self, worktree: &Path, blob: Option<&str>) -> Result<Vec<u8>, GitError> {
        let Some(blob) = blob else {
            return Ok(Vec::new());
        };
        self.run_hardened_bytes(worktree, &["cat-file", "blob", blob], NETWORK_TIMEOUT)
            .await
    }

    /// Git's hunks between two blobs. Blobs have no path, so no attribute can
    /// pick a diff command; every setting that changes the hunks is fixed
    /// here, so the user's `diff.*` config doesn't either.
    async fn hunks(&self, worktree: &Path, old: &str, new: &str) -> Result<Vec<Hunk>, GitError> {
        let diff = [
            "diff",
            "-U0",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--diff-algorithm=myers",
            "--indent-heuristic",
            "--inter-hunk-context=0",
            "--end-of-options",
            old,
            new,
        ];
        let output = self
            .run_hardened_bytes(worktree, &diff, NETWORK_TIMEOUT)
            .await?;
        parse_hunks(&output).ok_or_else(|| unexpected("diff"))
    }
}

fn unexpected(command: &str) -> GitError {
    GitError::Failed(format!("unexpected {command} output"))
}

/// The one hunk of a file that exists on one side only, as git gives it:
/// none when the file is empty.
fn whole(contents: &[u8], hunk: impl Fn(u64) -> Hunk) -> Vec<Hunk> {
    let breaks = contents.iter().filter(|byte| **byte == b'\n').count() as u64;
    let lines = breaks + u64::from(contents.last().is_some_and(|byte| *byte != b'\n'));
    if lines == 0 {
        Vec::new()
    } else {
        vec![hunk(lines)]
    }
}

/// The `@@ -a,b +c,d @@` headers of a unified diff. Every content line starts
/// with a space, `+`, `-` or `\`, so none can pass for one.
fn parse_hunks(output: &[u8]) -> Option<Vec<Hunk>> {
    let side = |part: &str| -> Option<(u64, u64)> {
        let (start, lines) = part.split_once(',').unwrap_or((part, "1"));
        Some((start.parse().ok()?, lines.parse().ok()?))
    };
    output
        .split(|byte| *byte == b'\n')
        .filter_map(|line| line.strip_prefix(b"@@ -"))
        .map(|line| {
            let line = std::str::from_utf8(line).ok()?;
            let mut parts = line.split(' ');
            let (old_start, old_lines) = side(parts.next()?)?;
            let (new_start, new_lines) = side(parts.next()?.strip_prefix('+')?)?;
            Some(Hunk {
                old_start,
                old_lines,
                new_start,
                new_lines,
            })
        })
        .collect()
}

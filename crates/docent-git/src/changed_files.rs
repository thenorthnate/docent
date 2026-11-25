//! The files a PR changes, from its merge base to its head (PLAN §5.4).

use std::path::Path;

use crate::runner::NETWORK_TIMEOUT;
use crate::worktree::pull_refs;
use crate::{Git, GitError, PrWorktreePath};

/// A file a PR changes. Paths come from the PR, so they are untrusted: the UI
/// shows them as text only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ChangedFile {
    /// Relative to the repo's top level. A name that isn't UTF-8 has its bad
    /// bytes replaced.
    pub path: String,
    /// Where a renamed file was before.
    pub old_path: Option<String>,
    pub status: FileStatus,
    /// `None` for a binary file, which git doesn't count lines in.
    pub lines: Option<LineCounts>,
    /// The file's contents before and after, as full object IDs; `None` on a
    /// side where it doesn't exist or is a submodule. These name the file exactly for
    /// [`Git::file_diff`], whatever its path.
    pub old_blob: Option<String>,
    pub new_blob: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Added,
    /// Contents, mode or type changed.
    Modified,
    Deleted,
    /// Moved, perhaps with edits too.
    Renamed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct LineCounts {
    pub added: u64,
    pub deleted: u64,
}

impl Git {
    /// The files that change between the merge base of `base_sha` and
    /// `head_sha`, and `head_sha`, in `clone`'s open worktree at `dest`. A base
    /// the clone doesn't have is fetched from `remote` first.
    pub async fn changed_files(
        &self,
        clone: &Path,
        remote: &str,
        dest: &PrWorktreePath,
        base_sha: &str,
        head_sha: &str,
    ) -> Result<Vec<ChangedFile>, GitError> {
        for sha in [base_sha, head_sha] {
            if !is_full_sha(sha) {
                return Err(GitError::InvalidSha(sha.to_string()));
            }
        }
        let worktree = self.checked_worktree(clone, dest).await?;
        // A partial clone fetches the objects it lacks as each of these asks for
        // them, so they get as long as a fetch.
        let commit = format!("{base_sha}^{{commit}}");
        if self
            .run_hardened(worktree, &["cat-file", "-e", &commit], NETWORK_TIMEOUT)
            .await
            .is_err()
        {
            let base_ref = format!("{}base", pull_refs(remote, dest.number()));
            let refspec = format!("+{base_sha}:{base_ref}");
            self.fetch_into(clone, remote, &refspec).await?;
        }
        let merge_base = self
            .run_hardened(
                worktree,
                &["merge-base", base_sha, head_sha],
                NETWORK_TIMEOUT,
            )
            .await?;
        // Plumbing, so the user's `diff.*` display settings don't apply, and
        // attributes in the PR can't pick out a configured diff command.
        let diff = [
            "diff-tree",
            "-r",
            "-M",
            "-z",
            "--raw",
            "--numstat",
            "--no-ext-diff",
            "--no-textconv",
            "--end-of-options",
            merge_base.trim(),
            head_sha,
        ];
        let output = self
            .run_hardened_bytes(worktree, &diff, NETWORK_TIMEOUT)
            .await?;
        parse(&output).ok_or_else(|| GitError::Failed("unexpected diff-tree output".to_string()))
    }

    /// `dest`'s folder, once it is checked to be inside Docent's worktrees
    /// folder and an existing worktree of `clone`.
    pub(crate) async fn checked_worktree<'a>(
        &self,
        clone: &Path,
        dest: &'a PrWorktreePath,
    ) -> Result<&'a Path, GitError> {
        dest.check_inside()?;
        let worktree = dest.path();
        // Without its `.git` file, git would act on whatever repo is above the
        // folder. The clone lists a worktree whose file is gone all the same.
        let linked = worktree.join(".git").is_file();
        if !linked || !self.is_worktree_of(clone, worktree).await? {
            return Err(GitError::WorktreeMissing(worktree.display().to_string()));
        }
        Ok(worktree)
    }
}

/// Whether `sha` is a whole SHA-1 or SHA-256 object name.
pub(crate) fn is_full_sha(sha: &str) -> bool {
    matches!(sha.len(), 40 | 64) && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Parses `diff-tree -z --raw --numstat`: every file's raw record, then every
/// file's counts in the same order. `None` if it isn't that.
fn parse(output: &[u8]) -> Option<Vec<ChangedFile>> {
    let mut fields = output
        .split(|byte| *byte == 0)
        .map(String::from_utf8_lossy)
        .peekable();
    let mut files = Vec::new();
    // `:100644 100644 <old> <new> R090`, then the old path if renamed, then
    // the path.
    while let Some(raw) = fields.next_if(|field| field.starts_with(':')) {
        let mut parts = raw[1..].split(' ');
        let (old_mode, new_mode) = (parts.next()?, parts.next()?);
        let old_blob = blob(old_mode, parts.next()?);
        let new_blob = blob(new_mode, parts.next()?);
        let status = match parts.next()?.chars().next()? {
            'A' => FileStatus::Added,
            'M' | 'T' => FileStatus::Modified,
            'D' => FileStatus::Deleted,
            'R' => FileStatus::Renamed,
            _ => return None,
        };
        let old_path = match status {
            FileStatus::Renamed => Some(fields.next()?.into_owned()),
            _ => None,
        };
        files.push(ChangedFile {
            path: fields.next()?.into_owned(),
            old_path,
            status,
            lines: None,
            old_blob,
            new_blob,
        });
    }
    // `<added>\t<deleted>\t<path>`, or an empty path and then both paths in
    // their own fields if renamed. Binary files have `-` for counts.
    for file in &mut files {
        let counts = fields.next()?;
        let mut counts = counts.splitn(3, '\t');
        let (added, deleted) = (counts.next()?, counts.next()?);
        if added != "-" {
            file.lines = Some(LineCounts {
                added: added.parse().ok()?,
                deleted: deleted.parse().ok()?,
            });
        }
        if counts.next()?.is_empty() {
            fields.nth(1)?;
        }
    }
    Some(files)
}

/// A side's object ID from a raw record, if it is a blob: not the all-zero ID
/// of a side the file doesn't exist on, nor a submodule's commit.
fn blob(mode: &str, id: &str) -> Option<String> {
    let absent = id.bytes().all(|byte| byte == b'0');
    (!absent && mode != "160000").then(|| id.to_string())
}

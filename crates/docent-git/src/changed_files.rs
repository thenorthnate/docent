//! The files a PR changes, from its merge base to its head (PLAN §5.4).

use crate::runner::TIMEOUT;
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
    /// `head_sha`, and `head_sha`, in the open worktree at `dest`. A base the
    /// clone doesn't have is fetched from `remote` first.
    pub async fn changed_files(
        &self,
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
        dest.check_inside()?;
        let worktree = dest.path();
        // A missing working directory fails to spawn like a missing `git`
        // would, so rule it out first.
        if !worktree.is_dir() {
            return Err(GitError::WorktreeMissing(worktree.display().to_string()));
        }
        let commit = format!("{base_sha}^{{commit}}");
        if self
            .run_hardened(worktree, &["cat-file", "-e", &commit], TIMEOUT)
            .await
            .is_err()
        {
            let base_ref = format!("{}base", pull_refs(remote, dest.number()));
            let refspec = format!("+{base_sha}:{base_ref}");
            self.fetch_into(worktree, remote, &refspec).await?;
        }
        let merge_base = self
            .run_hardened(worktree, &["merge-base", base_sha, head_sha], TIMEOUT)
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
        let output = self.run_hardened_bytes(dest.path(), &diff, TIMEOUT).await?;
        parse(&output).ok_or_else(|| GitError::Failed("unexpected diff-tree output".to_string()))
    }
}

/// Whether `sha` is a whole SHA-1 or SHA-256 object name.
fn is_full_sha(sha: &str) -> bool {
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
        let status = match raw.rsplit(' ').next()?.chars().next()? {
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

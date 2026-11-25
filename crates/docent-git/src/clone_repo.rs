//! Cloning a forge repo with the user's own git credentials (PLAN §4, "Repo
//! binding").

use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use crate::worktree::NO_REPO_COMMANDS;
use crate::{BoundClone, Git, GitError};

/// How far one phase of a clone has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneProgress {
    /// What git is doing, such as `Receiving objects`.
    pub phase: String,
    pub percent: u8,
}

impl Git {
    /// Makes a full clone of `host:owner/name` in a new folder `name` in
    /// `parent`. Git finds its own credentials; none are given to it.
    ///
    /// An existing folder is refused. If the clone fails, the new folder is
    /// deleted again.
    pub async fn clone_repo(
        &self,
        parent: &Path,
        host: &str,
        owner: &str,
        name: &str,
        mut on_progress: impl FnMut(CloneProgress),
    ) -> Result<BoundClone, GitError> {
        // The forge chose these, and they become a folder and a URL.
        for part in [host, owner, name] {
            if !is_plain_name(part) {
                return Err(GitError::InvalidName(part.to_string()));
            }
        }
        let dest = parent.join(name);
        let dest_str = dest
            .to_str()
            .ok_or_else(|| GitError::InvalidPath(dest.display().to_string()))?;
        // Creating the folder here, rather than leaving it to git, is what
        // makes it safe to delete: it fails if anything is already there.
        std::fs::create_dir(&dest).map_err(|err| match err.kind() {
            ErrorKind::AlreadyExists => GitError::FolderExists(dest.display().to_string()),
            _ => GitError::CreateFailed {
                path: dest.display().to_string(),
                why: err.to_string(),
            },
        })?;

        // Always https, so a user of ssh redirects it with `insteadOf`.
        let url = format!("https://{host}/{owner}/{name}.git");
        let cloned = self.clone_into(&dest, dest_str, &url, &mut on_progress);
        let checked = match cloned.await {
            Ok(()) => self.check_clone(&dest, host, owner, name).await,
            Err(err) => Err(explain(err, host)),
        };
        if checked.is_err() {
            delete(&dest).await;
        }
        checked
    }

    async fn clone_into(
        &self,
        dest: &Path,
        dest_str: &str,
        url: &str,
        on_progress: &mut impl FnMut(CloneProgress),
    ) -> Result<(), GitError> {
        // A relative hooks path in the user's config would run files from the
        // repo being cloned.
        let mut args = NO_REPO_COMMANDS.to_vec();
        args.extend([
            "clone",
            "--progress",
            "--no-recurse-submodules",
            "--",
            url,
            dest_str,
        ]);
        self.run_watched(dest, &args, |line| match parse_progress(line) {
            Some(progress) => {
                on_progress(progress);
                true
            }
            None => false,
        })
        .await
    }
}

/// Whether `part` is safe as a folder name and as a part of a URL.
fn is_plain_name(part: &str) -> bool {
    !matches!(part, "" | "." | "..")
        && !part.eq_ignore_ascii_case(".git")
        && !part.starts_with('-')
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Says what to do when git couldn't find credentials.
fn explain(err: GitError, host: &str) -> GitError {
    match err {
        GitError::Failed(why)
            if why.contains("could not read Username")
                || why.contains("could not read Password")
                || why.contains("Authentication failed") =>
        {
            GitError::NoCredentials {
                host: host.to_string(),
                why,
            }
        }
        other => other,
    }
}

/// Deletes a failed clone. Processes git started may still be writing to it
/// for a moment, so a failed delete is tried again.
async fn delete(dest: &Path) {
    let mut last = None;
    for _ in 0..5 {
        match std::fs::remove_dir_all(dest) {
            Ok(()) => return,
            // git deletes what it can itself, sometimes the folder too.
            Err(err) if err.kind() == ErrorKind::NotFound => return,
            Err(err) => last = Some(err),
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    if let Some(err) = last {
        eprintln!(
            "Could not delete the failed clone at {}: {err}",
            dest.display()
        );
    }
}

/// Reads `Receiving objects:  45% (9/20), 1.2 MiB | 3.0 MiB/s`, which git
/// prefixes with `remote: ` when the forge is the one working.
fn parse_progress(line: &str) -> Option<CloneProgress> {
    let line = line.strip_prefix("remote:").unwrap_or(line);
    let (phase, rest) = line.split_once(':')?;
    let (percent, rest) = rest.trim_start().split_once('%')?;
    if !rest.starts_with(" (") {
        return None;
    }
    let percent = percent.parse().ok().filter(|p| *p <= 100)?;
    Some(CloneProgress {
        phase: phase.trim().to_string(),
        percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_gits_progress_lines() {
        for (line, phase, percent) in [
            (
                "Receiving objects:  45% (9/20), 1.20 MiB | 3.00 MiB/s",
                "Receiving objects",
                45,
            ),
            (
                "remote: Counting objects: 100% (3/3), done.",
                "Counting objects",
                100,
            ),
            ("Updating files:   0% (1/900)", "Updating files", 0),
        ] {
            let phase = phase.to_string();
            assert_eq!(parse_progress(line), Some(CloneProgress { phase, percent }));
        }
    }

    #[test]
    fn other_lines_are_not_progress() {
        for line in [
            "Cloning into '/x'...",
            "fatal: could not read Username for 'https://github.com'",
            "remote: Enumerating objects: 5, done.",
            "remote: Disk: 101% (full)",
            "warning: 50% of quota",
            "fatal: 50% of it is missing",
        ] {
            assert_eq!(parse_progress(line), None, "{line}");
        }
    }
}

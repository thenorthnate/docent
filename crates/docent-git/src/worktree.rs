//! PR worktrees (PLAN §5.7): a PR's head, fetched into `refs/docent/*` and
//! checked out in detached HEAD in a folder Docent owns.

use std::path::{Path, PathBuf};

use crate::runner::NETWORK_TIMEOUT;
use crate::{Git, GitError};

/// A PR's worktree, checked out at its head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrWorktree {
    pub path: PathBuf,
    pub head_sha: String,
}

impl Git {
    /// Fetches PR `number`'s head from `remote` of `clone` and checks it out
    /// in detached HEAD at `dest`.
    pub async fn open_pr_worktree(
        &self,
        clone: &Path,
        remote: &str,
        number: u64,
        dest: &Path,
    ) -> Result<PrWorktree, GitError> {
        let head_ref = format!("refs/docent/{remote}/pull/{number}/head");
        let refspec = format!("+refs/pull/{number}/head:{head_ref}");
        self.run_for(
            clone,
            &["fetch", "--no-tags", "--no-write-fetch-head", remote, &refspec],
            NETWORK_TIMEOUT,
        )
        .await?;
        let sha = self
            .run(clone, &["rev-parse", "--verify", &format!("{head_ref}^{{commit}}")])
            .await?
            .trim()
            .to_string();
        let dest_str = dest.to_str().ok_or_else(|| {
            GitError::Failed(format!("{} isn't valid UTF-8", dest.display()))
        })?;
        self.run(clone, &["worktree", "add", "--detach", dest_str, &sha])
            .await?;
        Ok(PrWorktree {
            path: dest.to_path_buf(),
            head_sha: sha,
        })
    }
}

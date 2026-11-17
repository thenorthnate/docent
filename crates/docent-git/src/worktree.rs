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
        // Pin this head under its own name, so it stays reachable for
        // interdiffs after the PR is force-pushed.
        let pin = format!("refs/docent/{remote}/pull/{number}/{sha}");
        self.run(clone, &["update-ref", &pin, &sha]).await?;

        let dest_str = dest.to_str().ok_or_else(|| {
            GitError::Failed(format!("{} isn't valid UTF-8", dest.display()))
        })?;
        if self.is_worktree_of(clone, dest).await? {
            self.run(dest, &["checkout", "--quiet", "--detach", "--force", &sha])
                .await?;
        } else {
            // `--force` re-adds a worktree whose folder was deleted but whose
            // registration remains.
            self.run(
                clone,
                &["worktree", "add", "--quiet", "--force", "--detach", dest_str, &sha],
            )
            .await?;
        }
        Ok(PrWorktree {
            path: dest.to_path_buf(),
            head_sha: sha,
        })
    }
}

impl Git {
    /// Whether `dest` is an existing worktree folder of `clone`.
    async fn is_worktree_of(&self, clone: &Path, dest: &Path) -> Result<bool, GitError> {
        let Ok(dest) = dest.canonicalize() else {
            return Ok(false);
        };
        let list = self
            .run(clone, &["worktree", "list", "--porcelain"])
            .await?;
        Ok(list
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .any(|path| Path::new(path).canonicalize().is_ok_and(|p| p == dest)))
    }
}

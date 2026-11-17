//! PR worktrees (PLAN §5.7): a PR's head, fetched into `refs/docent/*` and
//! checked out in detached HEAD in a folder Docent owns.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::runner::{NETWORK_TIMEOUT, TIMEOUT};
use crate::{Git, GitError};

/// Config for every git call here. The clone's own config may name hooks,
/// an fsmonitor or submodules, and PR content is hostile, so none of them
/// may run: opening a PR runs only git itself.
const NO_REPO_COMMANDS: [&str; 10] = [
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "core.fsmonitor=false",
    "-c",
    "gc.auto=0",
    "-c",
    "maintenance.auto=false",
    "-c",
    "submodule.recurse=false",
];

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
        self.run_hardened(
            clone,
            &[
                "fetch",
                "--no-tags",
                "--no-write-fetch-head",
                "--no-recurse-submodules",
                // An empty refmap stops the remote's configured refspecs (say,
                // `refs/pull/*` into `refs/remotes/`) updating the user's refs.
                "--refmap=",
                "--end-of-options",
                remote,
                &refspec,
            ],
            NETWORK_TIMEOUT,
        )
        .await?;
        let sha = self
            .run_hardened(
                clone,
                &["rev-parse", "--verify", &format!("{head_ref}^{{commit}}")],
                TIMEOUT,
            )
            .await?
            .trim()
            .to_string();
        // Pin this head under its own name, so it stays reachable for
        // interdiffs after the PR is force-pushed.
        let pin = format!("refs/docent/{remote}/pull/{number}/{sha}");
        self.run_hardened(clone, &["update-ref", &pin, &sha], TIMEOUT)
            .await?;

        let dest_str = dest
            .to_str()
            .ok_or_else(|| GitError::Failed(format!("{} isn't valid UTF-8", dest.display())))?;
        if self.is_worktree_of(clone, dest).await? {
            self.run_hardened(
                dest,
                &["checkout", "--quiet", "--detach", "--force", &sha],
                TIMEOUT,
            )
            .await?;
        } else {
            // `--force` re-adds a worktree whose folder was deleted but whose
            // registration remains.
            self.run_hardened(
                clone,
                &[
                    "worktree", "add", "--quiet", "--force", "--detach", dest_str, &sha,
                ],
                TIMEOUT,
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
    /// Runs `git <args>` with [`NO_REPO_COMMANDS`] in front.
    async fn run_hardened(
        &self,
        cwd: &Path,
        args: &[&str],
        timeout: Duration,
    ) -> Result<String, GitError> {
        let all: Vec<&str> = NO_REPO_COMMANDS.iter().chain(args).copied().collect();
        self.run_for(cwd, &all, timeout).await
    }

    /// Whether `dest` is an existing worktree folder of `clone`.
    async fn is_worktree_of(&self, clone: &Path, dest: &Path) -> Result<bool, GitError> {
        let Ok(dest) = dest.canonicalize() else {
            return Ok(false);
        };
        let list = self
            .run_hardened(clone, &["worktree", "list", "--porcelain"], TIMEOUT)
            .await?;
        Ok(list
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .any(|path| Path::new(path).canonicalize().is_ok_and(|p| p == dest)))
    }
}

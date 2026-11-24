//! PR worktrees (PLAN §5.7): a PR's head, fetched into `refs/docent/*` and
//! checked out in detached HEAD in a folder Docent owns, and removed
//! again along with its pins.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::runner::{NETWORK_TIMEOUT, TIMEOUT};
use crate::worktrees_root::{PrWorktreePath, resolve};
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
    /// Fetches the head of `dest`'s PR from `remote` of `clone` and checks it
    /// out in detached HEAD at `dest`.
    pub async fn open_pr_worktree(
        &self,
        clone: &Path,
        remote: &str,
        dest: &PrWorktreePath,
    ) -> Result<PrWorktree, GitError> {
        dest.check_inside()?;
        let number = dest.number();
        let dest_str = utf8(dest.path())?;
        let head_ref = format!("{}head", pull_refs(remote, number));
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
        let pin = format!("{}{sha}", pull_refs(remote, number));
        self.run_hardened(clone, &["update-ref", &pin, &sha], TIMEOUT)
            .await?;

        if self.is_worktree_of(clone, dest.path()).await? {
            // The worktree is Docent's and read-only, so `--force` throws away
            // anything written there rather than refusing to move.
            self.run_hardened(
                dest.path(),
                &["checkout", "--quiet", "--detach", "--force", &sha],
                TIMEOUT,
            )
            .await?;
        } else {
            // A folder git doesn't list (say, from a crash mid-create) would
            // make `worktree add` fail with "already exists".
            dest.clear_stray()?;
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
            path: dest.path().to_path_buf(),
            head_sha: sha,
        })
    }

    /// Removes the worktree at `dest` and its PR's
    /// `refs/docent/<remote>/pull/<number>/*` pins.
    pub async fn remove_pr_worktree(
        &self,
        clone: &Path,
        remote: &str,
        dest: &PrWorktreePath,
    ) -> Result<(), GitError> {
        dest.check_inside()?;
        // With the clone gone, so are its registration and pins.
        if !clone.exists() {
            return dest.clear_stray();
        }
        let number = dest.number();
        let dest_str = utf8(dest.path())?;
        if self.lists_worktree(clone, dest.path()).await? {
            // `--force`: the worktree is Docent's, so anything written there
            // goes with it.
            self.run_hardened(clone, &["worktree", "remove", "--force", dest_str], TIMEOUT)
                .await?;
        } else {
            dest.clear_stray()?;
        }
        // The trailing slash matches whole path components, so PR 7's pins
        // never include PR 70's.
        let pins = pull_refs(remote, number);
        let refs = self
            .run_hardened(
                clone,
                &["for-each-ref", "--format=%(refname)", &pins],
                TIMEOUT,
            )
            .await?;
        for name in refs.lines() {
            self.run_hardened(clone, &["update-ref", "-d", name], TIMEOUT)
                .await?;
        }
        Ok(())
    }

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
        Ok(dest.is_dir() && self.lists_worktree(clone, dest).await?)
    }

    /// Whether `clone` lists a worktree at `dest`, even one whose folder is
    /// gone.
    async fn lists_worktree(&self, clone: &Path, dest: &Path) -> Result<bool, GitError> {
        let Some(dest) = resolve(dest) else {
            return Ok(false);
        };
        let list = self
            .run_hardened(clone, &["worktree", "list", "--porcelain"], TIMEOUT)
            .await?;
        Ok(list
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .any(|path| resolve(Path::new(path)).is_some_and(|p| p == dest)))
    }
}

/// The folder of refs Docent keeps for PR `number` of `remote`, with its
/// trailing slash.
fn pull_refs(remote: &str, number: u64) -> String {
    format!("refs/docent/{remote}/pull/{number}/")
}

/// `path` as a `&str`, since git's arguments here must be UTF-8.
fn utf8(path: &Path) -> Result<&str, GitError> {
    path.to_str()
        .ok_or_else(|| GitError::InvalidPath(path.display().to_string()))
}

//! PR worktrees (PLAN §5.7): a PR's head, fetched into `refs/docent/*` and
//! checked out in detached HEAD in a folder Docent owns.

use std::path::{Component, Path, PathBuf};
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
    /// in detached HEAD at `dest`, inside `root`, Docent's worktrees folder.
    pub async fn open_pr_worktree(
        &self,
        clone: &Path,
        remote: &str,
        number: u64,
        _root: &Path,
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

        let dest_str = utf8(dest)?;
        if self.is_worktree_of(clone, dest).await? {
            // The worktree is Docent's and read-only, so `--force` throws away
            // anything written there rather than refusing to move.
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

    /// Removes PR `number`'s worktree at `dest`, inside `root`, and its
    /// `refs/docent/<remote>/pull/<number>/*` pins.
    pub async fn remove_pr_worktree(
        &self,
        clone: &Path,
        remote: &str,
        number: u64,
        root: &Path,
        dest: &Path,
    ) -> Result<(), GitError> {
        check_inside(root, dest)?;
        let dest_str = utf8(dest)?;
        if self.lists(clone, dest).await? {
            // `--force`: the worktree is Docent's, so anything written there
            // goes with it.
            self.run_hardened(clone, &["worktree", "remove", "--force", dest_str], TIMEOUT)
                .await?;
        }
        // The trailing slash matches whole path components, so PR 7's pins
        // never include PR 70's.
        let pins = format!("refs/docent/{remote}/pull/{number}/");
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
        Ok(dest.is_dir() && self.lists(clone, dest).await?)
    }

    /// Whether `clone` lists a worktree at `dest`, even one whose folder is
    /// gone.
    async fn lists(&self, clone: &Path, dest: &Path) -> Result<bool, GitError> {
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

/// `path` as a `&str`, since git's arguments here must be UTF-8.
fn utf8(path: &Path) -> Result<&str, GitError> {
    path.to_str()
        .ok_or_else(|| GitError::InvalidPath(path.display().to_string()))
}

/// `path` with symlinks resolved, as far as it exists: a missing last
/// component is kept as given.
fn resolve(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = path.canonicalize() {
        return Some(path);
    }
    Some(path.parent()?.canonicalize().ok()?.join(path.file_name()?))
}

/// Refuses a `dest` that isn't strictly inside `root`, so nothing outside
/// Docent's worktrees folder is ever removed: not by a `..`, not through a
/// symlink, and not `root` itself.
fn check_inside(root: &Path, dest: &Path) -> Result<(), GitError> {
    let refuse = || Err(GitError::OutsideWorktrees(dest.display().to_string()));
    let lexically_inside = dest.starts_with(root)
        && dest != root
        && !dest.components().any(|c| c == Component::ParentDir);
    let is_symlink = dest
        .symlink_metadata()
        .is_ok_and(|meta| meta.file_type().is_symlink());
    if !lexically_inside || is_symlink {
        return refuse();
    }
    // A symlinked folder on the way may still lead outside. A `dest` that
    // doesn't resolve has no folder to remove.
    if let Some(resolved) = resolve(dest) {
        match resolve(root) {
            Some(root) if resolved.starts_with(&root) && resolved != root => {}
            _ => return refuse(),
        }
    }
    Ok(())
}

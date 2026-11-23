/// A git failure, worded for the UI: each message says what went wrong and,
/// where there is one, what to do.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitError {
    #[error("git wasn't found. Install it, or make sure it's on your login shell's PATH.")]
    NotFound,
    #[error("{0} isn't a git clone. Pick the folder you cloned the repo into.")]
    NotARepo(String),
    #[error("{}", no_match_message(.repo, .found))]
    NoMatchingRemote {
        /// The repo looked for, as `host:owner/name`.
        repo: String,
        /// The clone's remote URLs, with any credentials removed.
        found: Vec<String>,
    },
    #[error("Docent can't put a worktree at {0}: git needs a UTF-8 path. Report this as a bug.")]
    InvalidPath(String),
    #[error(
        "Docent only removes worktrees in its own worktrees folder, and {0} isn't in it. Report this as a bug."
    )]
    OutsideWorktrees(String),
    #[error("git failed: {0}")]
    Failed(String),
}

fn no_match_message(repo: &str, found: &[String]) -> String {
    if found.is_empty() {
        format!("This clone has no remotes, so it can't be checked against {repo}.")
    } else {
        format!(
            "None of this clone's remotes point at {repo}. Its remotes are: {}.",
            found.join(", ")
        )
    }
}

impl GitError {
    /// A stable name for the UI to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotFound => "git_not_found",
            Self::NotARepo(_) => "not_a_repo",
            Self::NoMatchingRemote { .. } => "no_matching_remote",
            Self::InvalidPath(_) => "invalid_path",
            Self::OutsideWorktrees(_) => "outside_worktrees",
            Self::Failed(_) => "git_failed",
        }
    }
}

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
    #[error("Docent couldn't delete the leftover folder at {path}: {why}")]
    ClearFailed { path: String, why: String },
    #[error("Docent couldn't measure the worktree at {path}: {why}")]
    MeasureFailed { path: String, why: String },
    #[error("{0:?} isn't a full commit SHA. Report this as a bug.")]
    InvalidSha(String),
    #[error("The PR's worktree at {0} is gone. Reopen the PR to make it again.")]
    WorktreeMissing(String),
    #[error("{0:?} isn't a name Docent can clone by. Clone the repo yourself, then use Bind.")]
    InvalidName(String),
    #[error(
        "{0} already exists. If it's your finished clone, use Bind. Otherwise pick another folder to clone into."
    )]
    FolderExists(String),
    #[error("Docent couldn't create the folder {path}: {why}")]
    CreateFailed { path: String, why: String },
    #[error(
        "git has no working credentials for {host}. Run `gh auth setup-git --hostname {host}`, or set up git's own, then try again. git said: {why}"
    )]
    NoCredentials { host: String, why: String },
    #[error(
        "Your git config redirects {host} to {url}, which Docent can't match to {host}. Change the insteadOf that does it, or clone the repo yourself."
    )]
    Redirected { host: String, url: String },
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
            Self::ClearFailed { .. } => "clear_failed",
            Self::MeasureFailed { .. } => "measure_failed",
            Self::InvalidSha(_) => "invalid_sha",
            Self::WorktreeMissing(_) => "worktree_missing",
            Self::InvalidName(_) => "invalid_name",
            Self::FolderExists(_) => "folder_exists",
            Self::CreateFailed { .. } => "create_failed",
            Self::NoCredentials { .. } => "no_credentials",
            Self::Redirected { .. } => "redirected",
            Self::Failed(_) => "git_failed",
        }
    }
}

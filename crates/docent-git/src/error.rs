#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitError {
    #[error("git wasn't found. Install it, or make sure it's on your login shell's PATH.")]
    NotFound,
    #[error("git failed: {0}")]
    Failed(String),
}

impl GitError {
    /// A stable name for the UI to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotFound => "git_not_found",
            Self::Failed(_) => "git_failed",
        }
    }
}

use crate::{AccountSummary, ConnectError};

/// A repo the forge confirmed exists, with its canonical spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeRepo {
    /// The connected account the repo was found with.
    pub account: AccountSummary,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
}

/// Why a repo could not be looked up. No message carries the token.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RepoError {
    #[error("{0:?} is not a repo. Enter it as owner/name, like octocat/Hello-World.")]
    InvalidName(String),
    #[error("{0} is not connected. Connect it in Settings → Accounts first.")]
    NotConnected(String),
    #[error("{host} has no repo {repo}, or your account can't see it.")]
    NotFound { host: String, repo: String },
    #[error(transparent)]
    Connect(#[from] ConnectError),
}

impl RepoError {
    /// A stable name for the UI to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidName(_) => "invalid_name",
            Self::NotConnected(_) => "not_connected",
            Self::NotFound { .. } => "not_found",
            Self::Connect(err) => err.kind(),
        }
    }
}

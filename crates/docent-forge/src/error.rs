use serde::ser::SerializeStruct;

/// Why a host could not be connected. Each message says what to do next, and
/// none carries the token.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ConnectError {
    #[error("{0:?} is not a host name. Enter one like github.com or github.example.com.")]
    InvalidHost(String),
    #[error(
        "Docent could not find `gh`. Install the GitHub CLI (https://cli.github.com), \
         run `gh auth login`, then try again."
    )]
    GhNotFound,
    #[error("Could not run `gh`: {0}")]
    GhFailed(String),
    #[error(
        "`gh` is not logged in to {0}. Run `gh auth login --hostname {0}` in a terminal, \
         then try again."
    )]
    NotLoggedIn(String),
    #[error(
        "{0} rejected the token from `gh`. Run `gh auth refresh --hostname {0}` in a terminal, \
         then try again."
    )]
    TokenRejected(String),
    #[error("{host} answered with HTTP {status}.")]
    HttpStatus { host: String, status: u16 },
    #[error("Could not reach {host}: {reason}")]
    Unreachable { host: String, reason: String },
}

impl ConnectError {
    /// A stable name for the UI to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidHost(_) => "invalid_host",
            Self::GhNotFound => "gh_not_found",
            Self::GhFailed(_) => "gh_failed",
            Self::NotLoggedIn(_) => "not_logged_in",
            Self::TokenRejected(_) => "token_rejected",
            Self::HttpStatus { .. } => "http_status",
            Self::Unreachable { .. } => "unreachable",
        }
    }
}

/// Sent to the UI as `{ "kind": ..., "message": ... }`.
impl serde::Serialize for ConnectError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ConnectError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

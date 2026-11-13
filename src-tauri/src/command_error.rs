//! The error type Tauri commands send to the UI.

use docent_forge::{ConnectError, RepoError};
use docent_store::StoreError;

/// An error sent to the UI as `{ "kind": ..., "message": ... }`, like
/// `ConnectError`. The message says what to do next.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct CommandError {
    pub(crate) kind: &'static str,
    pub(crate) message: String,
}

impl From<RepoError> for CommandError {
    fn from(err: RepoError) -> Self {
        Self {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

impl From<StoreError> for CommandError {
    fn from(err: StoreError) -> Self {
        Self {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

impl From<ConnectError> for CommandError {
    fn from(err: ConnectError) -> Self {
        Self {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_reach_the_ui_as_kind_and_message() {
        let err = CommandError::from(StoreError::AlreadyWatched(
            "github.com:octocat/Hello-World".to_string(),
        ));

        assert_eq!(
            serde_json::to_value(&err).unwrap(),
            serde_json::json!({
                "kind": "already_watched",
                "message": "github.com:octocat/Hello-World is already watched.",
            })
        );
    }
}

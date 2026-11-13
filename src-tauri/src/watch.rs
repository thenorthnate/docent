//! Watching a repo: check it exists on the forge, then remember it.

use docent_forge::{Accounts, RepoError, RepoName};
use docent_store::{NewRepo, Store, StoreError, WatchedRepo};

/// An error sent to the UI as `{ "kind": ..., "message": ... }`, like
/// `ConnectError`. The message says what to do next.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct CommandError {
    kind: &'static str,
    message: String,
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

/// Watches `repo` (`owner/name`) on the connected `host`.
pub async fn watch(
    accounts: &Accounts,
    store: &Store,
    host: &str,
    repo: &str,
) -> Result<WatchedRepo, CommandError> {
    let name = RepoName::parse(repo)?;
    let found = accounts.lookup_repo(host, &name).await?;
    // Host and login only: the store has nowhere to put a token.
    let account = store
        .upsert_account(&found.account.host, &found.account.login)
        .await?;
    let watched = store
        .watch_repo(
            account,
            &NewRepo {
                owner: found.owner,
                name: found.name,
                default_branch: found.default_branch,
            },
        )
        .await?;
    Ok(watched)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> Store {
        Store::open_in_memory().await.expect("store")
    }

    #[tokio::test]
    async fn a_malformed_name_is_rejected_before_any_lookup() {
        let accounts = Accounts::with_api_base(|_| unreachable!("no lookup for a bad name"));

        let err = watch(&accounts, &store().await, "github.com", "octocat")
            .await
            .unwrap_err();

        assert_eq!(err.kind, "invalid_name");
        assert!(err.message.contains("owner/name"), "{}", err.message);
    }

    #[tokio::test]
    async fn a_host_that_is_not_connected_says_to_connect_it() {
        let accounts = Accounts::with_api_base(|_| unreachable!("no account, no lookup"));
        let store = store().await;

        let err = watch(&accounts, &store, "github.com", "octocat/Hello-World")
            .await
            .unwrap_err();

        assert_eq!(err.kind, "not_connected");
        assert!(err.message.contains("Settings"), "{}", err.message);
        assert_eq!(store.watched_repos().await.unwrap(), vec![]);
    }

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

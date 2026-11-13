//! Watching a repo: check it exists on the forge, then remember it.

use docent_forge::Accounts;
use docent_forge::github::RepoName;
use docent_store::{NewRepo, Store, WatchedRepo};

use crate::command_error::CommandError;

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
}

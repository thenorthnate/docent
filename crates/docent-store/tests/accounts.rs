//! Remembering connected hosts, through the store's public API.

use docent_store::{SavedAccount, Store};

fn saved(host: &str, login: &str) -> SavedAccount {
    SavedAccount {
        host: host.to_string(),
        login: login.to_string(),
    }
}

#[tokio::test]
async fn a_new_store_has_no_saved_accounts() {
    let store = Store::open_in_memory().await.expect("open");

    assert_eq!(store.saved_accounts().await.expect("list"), vec![]);
}

#[tokio::test]
async fn saved_accounts_are_listed_by_host_with_their_latest_login() {
    let store = Store::open_in_memory().await.expect("open");

    store.upsert_account("github.com", "octocat").await.unwrap();
    store.upsert_account("ghe.corp.test", "mona").await.unwrap();
    store.upsert_account("github.com", "hubot").await.unwrap();

    assert_eq!(
        store.saved_accounts().await.unwrap(),
        vec![saved("ghe.corp.test", "mona"), saved("github.com", "hubot")]
    );
}

#[tokio::test]
async fn saved_accounts_survive_reopening_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("docent.sqlite");
    {
        let store = Store::open(&path).await.expect("create");
        store.upsert_account("github.com", "octocat").await.unwrap();
        store.close().await;
    }

    let reopened = Store::open(&path).await.expect("reopen");

    assert_eq!(
        reopened.saved_accounts().await.unwrap(),
        vec![saved("github.com", "octocat")]
    );
}

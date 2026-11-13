//! Watching repos, through the store's public API.

use docent_store::{NewRepo, Store, StoreError, WatchedRepo};

fn repo(owner: &str, name: &str) -> NewRepo {
    NewRepo {
        owner: owner.to_string(),
        name: name.to_string(),
        default_branch: "main".to_string(),
    }
}

fn names(repos: &[WatchedRepo]) -> Vec<String> {
    repos
        .iter()
        .map(|r| format!("{}:{}/{}", r.host, r.owner, r.name))
        .collect()
}

#[tokio::test]
async fn a_new_store_has_no_watched_repos() {
    let store = Store::open_in_memory().await.expect("open");

    assert_eq!(store.watched_repos().await.expect("list"), vec![]);
}

#[tokio::test]
async fn a_watched_repo_is_listed_with_its_host_and_default_branch() {
    let store = Store::open_in_memory().await.expect("open");
    let account = store
        .upsert_account("github.com", "octocat")
        .await
        .expect("account");

    let watched = store
        .watch_repo(account, &repo("octocat", "Hello-World"))
        .await
        .expect("watch");

    assert_eq!(watched.host, "github.com");
    assert_eq!(watched.owner, "octocat");
    assert_eq!(watched.name, "Hello-World");
    assert_eq!(watched.default_branch, "main");
    assert_eq!(watched.local_path, None);
    assert_eq!(store.watched_repos().await.expect("list"), vec![watched]);
}

#[tokio::test]
async fn repos_are_listed_by_host_then_owner_then_name() {
    let store = Store::open_in_memory().await.expect("open");
    let public = store.upsert_account("github.com", "octocat").await.unwrap();
    let corp = store.upsert_account("ghe.corp.test", "mona").await.unwrap();

    store.watch_repo(public, &repo("octocat", "zeta")).await.unwrap();
    store.watch_repo(corp, &repo("team", "tool")).await.unwrap();
    store.watch_repo(public, &repo("octocat", "Alpha")).await.unwrap();

    assert_eq!(
        names(&store.watched_repos().await.unwrap()),
        [
            "ghe.corp.test:team/tool",
            "github.com:octocat/Alpha",
            "github.com:octocat/zeta"
        ]
    );
}

#[tokio::test]
async fn upserting_an_account_again_keeps_one_row_per_host() {
    let store = Store::open_in_memory().await.expect("open");

    let first = store.upsert_account("github.com", "octocat").await.unwrap();
    let again = store.upsert_account("github.com", "octocat2").await.unwrap();

    assert_eq!(first, again);
}

#[tokio::test]
async fn watching_a_repo_twice_is_rejected() {
    let store = Store::open_in_memory().await.expect("open");
    let account = store.upsert_account("github.com", "octocat").await.unwrap();
    store
        .watch_repo(account, &repo("octocat", "Hello-World"))
        .await
        .unwrap();

    // GitHub names are case-insensitive, so a case-only difference is the
    // same repo.
    for dupe in [repo("octocat", "Hello-World"), repo("OctoCat", "hello-world")] {
        let err = store.watch_repo(account, &dupe).await.unwrap_err();
        assert_eq!(
            err,
            StoreError::AlreadyWatched("github.com:octocat/Hello-World".to_string())
        );
    }
    assert_eq!(store.watched_repos().await.unwrap().len(), 1);
}

#[tokio::test]
async fn the_same_repo_name_on_two_hosts_is_two_repos() {
    let store = Store::open_in_memory().await.expect("open");
    let public = store.upsert_account("github.com", "octocat").await.unwrap();
    let corp = store.upsert_account("ghe.corp.test", "mona").await.unwrap();

    store.watch_repo(public, &repo("octocat", "tool")).await.unwrap();
    store.watch_repo(corp, &repo("octocat", "tool")).await.unwrap();

    assert_eq!(store.watched_repos().await.unwrap().len(), 2);
}

#[tokio::test]
async fn an_unwatched_repo_leaves_the_list_and_can_be_watched_again() {
    let store = Store::open_in_memory().await.expect("open");
    let account = store.upsert_account("github.com", "octocat").await.unwrap();
    let watched = store
        .watch_repo(account, &repo("octocat", "Hello-World"))
        .await
        .unwrap();

    store.unwatch_repo(watched.id).await.expect("unwatch");
    assert_eq!(store.watched_repos().await.unwrap(), vec![]);

    let again = NewRepo {
        default_branch: "trunk".to_string(),
        ..repo("octocat", "Hello-World")
    };
    let rewatched = store.watch_repo(account, &again).await.expect("re-watch");
    assert_eq!(rewatched.id, watched.id, "the same row comes back");
    assert_eq!(rewatched.default_branch, "trunk");
    assert_eq!(store.watched_repos().await.unwrap(), vec![rewatched]);
}

#[tokio::test]
async fn unwatching_an_unknown_repo_is_an_error() {
    let store = Store::open_in_memory().await.expect("open");

    let err = store.unwatch_repo(42).await.unwrap_err();

    assert_eq!(err, StoreError::RepoNotFound(42));
}

#[tokio::test]
async fn watched_repos_survive_reopening_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("nested").join("docent.sqlite");
    {
        let store = Store::open(&path).await.expect("create");
        let account = store.upsert_account("github.com", "octocat").await.unwrap();
        store
            .watch_repo(account, &repo("octocat", "Hello-World"))
            .await
            .unwrap();
        store.close().await;
    }

    let reopened = Store::open(&path).await.expect("reopen");

    assert_eq!(
        names(&reopened.watched_repos().await.unwrap()),
        ["github.com:octocat/Hello-World"]
    );
}

//! Binding watched repos to local clones, through the store's public API.

use docent_store::{NewRepo, RepoId, Store, StoreError};

async fn watched(store: &Store, name: &str) -> RepoId {
    let account = store.upsert_account("github.com", "octocat").await.unwrap();
    let repo = NewRepo {
        owner: "octocat".to_string(),
        name: name.to_string(),
        default_branch: "main".to_string(),
    };
    store.watch_repo(account, &repo).await.unwrap().id
}

#[tokio::test]
async fn a_watched_repo_can_be_looked_up_by_id() {
    let store = Store::open_in_memory().await.unwrap();
    let id = watched(&store, "Hello-World").await;

    let repo = store.watched_repo(id).await.expect("found");

    assert_eq!(
        (repo.host.as_str(), repo.name.as_str()),
        ("github.com", "Hello-World")
    );
}

#[tokio::test]
async fn binding_records_the_clone_path() {
    let store = Store::open_in_memory().await.unwrap();
    let id = watched(&store, "Hello-World").await;

    let bound = store
        .bind_repo(id, "/Users/me/code/hello")
        .await
        .expect("bind");

    assert_eq!(bound.local_path.as_deref(), Some("/Users/me/code/hello"));
    assert_eq!(store.watched_repos().await.unwrap(), vec![bound]);
}

#[tokio::test]
async fn unknown_and_unwatched_repos_cannot_be_bound_or_looked_up() {
    let store = Store::open_in_memory().await.unwrap();
    let id = watched(&store, "Hello-World").await;
    store.unwatch_repo(id).await.unwrap();

    for id in [id, 999] {
        assert_eq!(
            store.watched_repo(id).await,
            Err(StoreError::RepoNotFound(id))
        );
        assert_eq!(
            store.bind_repo(id, "/Users/me/code/hello").await,
            Err(StoreError::RepoNotFound(id))
        );
    }
}

#[tokio::test]
async fn a_binding_survives_reopening_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("docent.sqlite");
    let store = Store::open(&path).await.unwrap();
    let id = watched(&store, "Hello-World").await;
    store.bind_repo(id, "/Users/me/code/hello").await.unwrap();
    store.close().await;

    let reopened = Store::open(&path).await.unwrap();

    let repo = reopened.watched_repo(id).await.unwrap();
    assert_eq!(repo.local_path.as_deref(), Some("/Users/me/code/hello"));
}

/// A route names a PR's repo by host, owner and name, in any case.
#[tokio::test]
async fn a_watched_repo_can_be_looked_up_by_name_in_any_case() {
    let store = Store::open_in_memory().await.unwrap();
    let id = watched(&store, "Hello-World").await;

    let repo = store
        .watched_repo_named("GitHub.com", "OctoCat", "hello-world")
        .await
        .expect("found");

    assert_eq!(repo.id, id);
}

#[tokio::test]
async fn an_unknown_or_unwatched_name_is_not_watched() {
    let store = Store::open_in_memory().await.unwrap();
    let id = watched(&store, "Hello-World").await;
    store.unwatch_repo(id).await.unwrap();

    for (host, name) in [
        ("github.com", "Hello-World"),
        ("github.com", "Spoon-Knife"),
        ("ghe.example.com", "Hello-World"),
    ] {
        let err = store
            .watched_repo_named(host, "octocat", name)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            StoreError::NotWatched(format!("{host}:octocat/{name}"))
        );
        assert_eq!(err.kind(), "not_watched");
    }
}

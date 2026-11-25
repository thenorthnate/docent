//! Recording PR worktrees and their disk cap, through the store's public API.

use docent_store::{DEFAULT_WORKTREE_CAP, NewRepo, RepoId, Store};

async fn watched(store: &Store, name: &str) -> RepoId {
    let account = store.upsert_account("github.com", "octocat").await.unwrap();
    let repo = NewRepo {
        owner: "octocat".to_string(),
        name: name.to_string(),
        default_branch: "main".to_string(),
    };
    store.watch_repo(account, &repo).await.unwrap().id
}

/// The recorded worktrees as `(repo, PR)`, in the store's order.
async fn recorded(store: &Store) -> Vec<(RepoId, u64)> {
    let worktrees = store.worktrees().await.expect("lists");
    worktrees.iter().map(|w| (w.repo_id, w.number)).collect()
}

/// Opened back to back, well within a millisecond of each other.
#[tokio::test]
async fn worktrees_are_listed_least_recently_opened_first() {
    let store = Store::open_in_memory().await.unwrap();
    let (a, b) = (watched(&store, "a").await, watched(&store, "b").await);
    for (repo, number) in [(b, 9), (a, 7), (a, 8), (b, 9)] {
        store
            .record_worktree(repo, number, "/code/x", "origin")
            .await
            .expect("records");
        store.touch_worktree(repo, number).await.expect("touches");
    }

    assert_eq!(recorded(&store).await, [(a, 7), (a, 8), (b, 9)]);
}

/// An open that fails is recorded but never touched, so it mustn't look like
/// the worktree in use.
#[tokio::test]
async fn recording_alone_does_not_count_as_opening() {
    let store = Store::open_in_memory().await.unwrap();
    let repo = watched(&store, "a").await;
    for number in [1, 2, 3] {
        store
            .record_worktree(repo, number, "/code/x", "origin")
            .await
            .unwrap();
    }
    for number in [1, 2] {
        store.touch_worktree(repo, number).await.unwrap();
    }

    store
        .record_worktree(repo, 1, "/code/x", "origin")
        .await
        .unwrap();

    assert_eq!(recorded(&store).await, [(repo, 3), (repo, 1), (repo, 2)]);
}

#[tokio::test]
async fn opening_again_from_another_clone_records_the_new_one() {
    let store = Store::open_in_memory().await.unwrap();
    let repo = watched(&store, "a").await;
    store
        .record_worktree(repo, 7, "/code/old", "origin")
        .await
        .unwrap();

    store
        .record_worktree(repo, 7, "/code/new", "upstream")
        .await
        .expect("records");

    let worktrees = store.worktrees().await.unwrap();
    assert_eq!(worktrees.len(), 1);
    assert_eq!(worktrees[0].clone_path, "/code/new");
    assert_eq!(worktrees[0].remote, "upstream");
}

/// Removing a worktree later needs the clone it came from.
#[tokio::test]
async fn a_worktree_keeps_its_clone_when_the_repo_is_rebound_or_unwatched() {
    let store = Store::open_in_memory().await.unwrap();
    let repo = watched(&store, "a").await;
    store.bind_repo(repo, "/code/old").await.unwrap();
    store
        .record_worktree(repo, 7, "/code/old", "origin")
        .await
        .unwrap();

    store.bind_repo(repo, "/code/new").await.unwrap();
    store.unwatch_repo(repo).await.unwrap();

    assert_eq!(store.worktrees().await.unwrap()[0].clone_path, "/code/old");
}

#[tokio::test]
async fn a_forgotten_worktree_is_no_longer_listed() {
    let store = Store::open_in_memory().await.unwrap();
    let repo = watched(&store, "a").await;
    for number in [7, 8] {
        store
            .record_worktree(repo, number, "/code/x", "origin")
            .await
            .unwrap();
    }

    // Forgetting twice is fine.
    for _ in 0..2 {
        store.forget_worktree(repo, 7).await.expect("forgets");
    }

    assert_eq!(recorded(&store).await, [(repo, 8)]);
}

#[tokio::test]
async fn the_cap_is_ten_gigabytes_until_set_and_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("docent.sqlite");
    let store = Store::open(&path).await.unwrap();
    assert_eq!(store.worktree_cap().await, Ok(10_000_000_000));
    assert_eq!(DEFAULT_WORKTREE_CAP, 10_000_000_000);

    for cap in [1, 5_000_000_000] {
        store.set_worktree_cap(cap).await.expect("sets");
    }
    store.close().await;

    let store = Store::open(&path).await.unwrap();
    assert_eq!(store.worktree_cap().await, Ok(5_000_000_000));
}

/// Eviction frees the disk but keeps the PR's pins, so the PR stays on
/// record, with its repo's name, until it is forgotten.
#[tokio::test]
async fn an_evicted_worktree_is_not_listed_but_its_pr_stays_on_record() {
    let store = Store::open_in_memory().await.unwrap();
    let repo = watched(&store, "a").await;
    for number in [7, 8] {
        store
            .record_worktree(repo, number, "/code/x", "origin")
            .await
            .unwrap();
    }

    store.mark_evicted(repo, 7).await.expect("marks");

    assert_eq!(recorded(&store).await, [(repo, 8)]);
    let pulls = store.recorded_pulls().await.expect("lists");
    let found: Vec<_> = pulls
        .iter()
        .map(|p| (p.host.as_str(), p.owner.as_str(), p.name.as_str(), p.worktree.number))
        .collect();
    assert_eq!(
        found,
        [("github.com", "octocat", "a", 7), ("github.com", "octocat", "a", 8)]
    );
    assert_eq!(pulls[0].worktree.clone_path, "/code/x");

    store.forget_worktree(repo, 7).await.unwrap();
    assert_eq!(store.recorded_pulls().await.unwrap().len(), 1);
}

#[tokio::test]
async fn opening_an_evicted_worktree_again_lists_it_again() {
    let store = Store::open_in_memory().await.unwrap();
    let repo = watched(&store, "a").await;
    store
        .record_worktree(repo, 7, "/code/x", "origin")
        .await
        .unwrap();
    store.mark_evicted(repo, 7).await.unwrap();

    store
        .record_worktree(repo, 7, "/code/x", "origin")
        .await
        .unwrap();

    assert_eq!(recorded(&store).await, [(repo, 7)]);
}

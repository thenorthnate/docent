//! The inbox's Review requested section: the forge's review requests, kept
//! to watched repos.

use docent_forge::{Forge, ReviewRequested};
use docent_store::Store;

use crate::command_error::CommandError;

/// Open PRs that request the user's review, in watched repos only.
pub async fn review_requested(
    forge: &impl Forge,
    store: &Store,
) -> Result<ReviewRequested, CommandError> {
    let _ = (forge, store);
    Ok(ReviewRequested::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use docent_forge::{ConnectError, FakeForge, HostFailure, PullSummary};
    use docent_store::NewRepo;

    async fn watching(repos: &[(&str, &str)]) -> Store {
        let store = Store::open_in_memory().await.expect("store");
        for (host, repo) in repos {
            let (owner, name) = repo.split_once('/').expect("owner/name");
            let account = store.upsert_account(host, "octocat").await.unwrap();
            let repo = NewRepo {
                owner: owner.to_string(),
                name: name.to_string(),
                default_branch: "main".to_string(),
            };
            store.watch_repo(account, &repo).await.unwrap();
        }
        store
    }

    fn pull(host: &str, repo: &str, number: u64, created_at: &str) -> PullSummary {
        let (owner, name) = repo.split_once('/').expect("owner/name");
        PullSummary {
            host: host.to_string(),
            owner: owner.to_string(),
            repo: name.to_string(),
            number,
            title: format!("PR {number}"),
            author: "mona".to_string(),
            created_at: created_at.to_string(),
        }
    }

    fn failure(host: &str) -> HostFailure {
        HostFailure {
            host: host.to_string(),
            error: ConnectError::HttpStatus {
                host: host.to_string(),
                status: 503,
            },
        }
    }

    fn forge(pulls: Vec<PullSummary>, failures: Vec<HostFailure>) -> FakeForge {
        FakeForge::with_review_requested(ReviewRequested { pulls, failures })
    }

    #[tokio::test]
    async fn only_prs_in_watched_repos_are_kept_newest_first() {
        let store = watching(&[("github.com", "octocat/Hello-World")]).await;
        let forge = forge(
            vec![
                pull(
                    "github.com",
                    "octocat/Hello-World",
                    1,
                    "2026-11-01T00:00:00Z",
                ),
                pull(
                    "github.com",
                    "octocat/Spoon-Knife",
                    2,
                    "2026-11-12T00:00:00Z",
                ),
                pull(
                    "github.com",
                    "octocat/Hello-World",
                    3,
                    "2026-11-10T00:00:00Z",
                ),
            ],
            vec![],
        );

        let found = review_requested(&forge, &store).await.unwrap();

        let numbers: Vec<u64> = found.pulls.iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![3, 1]);
    }

    #[tokio::test]
    async fn repo_names_match_ignoring_case_as_github_does() {
        let store = watching(&[("github.com", "octocat/Hello-World")]).await;
        let forge = forge(
            vec![pull(
                "github.com",
                "OctoCat/hello-world",
                1,
                "2026-11-01T00:00:00Z",
            )],
            vec![],
        );

        let found = review_requested(&forge, &store).await.unwrap();

        assert_eq!(found.pulls.len(), 1);
    }

    #[tokio::test]
    async fn the_same_repo_name_on_another_host_is_left_out() {
        let store = watching(&[("github.com", "octocat/Hello-World")]).await;
        let forge = forge(
            vec![pull(
                "ghe.corp.test",
                "octocat/Hello-World",
                1,
                "2026-11-01T00:00:00Z",
            )],
            vec![],
        );

        let found = review_requested(&forge, &store).await.unwrap();

        assert_eq!(found.pulls, vec![]);
    }

    #[tokio::test]
    async fn with_no_watched_repos_the_forge_is_not_asked() {
        let store = watching(&[]).await;
        let forge = forge(
            vec![pull(
                "github.com",
                "octocat/Hello-World",
                1,
                "2026-11-01T00:00:00Z",
            )],
            vec![failure("github.com")],
        );

        let found = review_requested(&forge, &store).await.unwrap();

        assert_eq!(found, ReviewRequested::default());
        assert_eq!(forge.calls(), 0);
    }

    #[tokio::test]
    async fn only_failures_on_hosts_with_watched_repos_are_shown() {
        let store = watching(&[("github.com", "octocat/Hello-World")]).await;
        let forge = forge(
            vec![],
            vec![failure("github.com"), failure("ghe.corp.test")],
        );

        let found = review_requested(&forge, &store).await.unwrap();

        assert_eq!(found.failures, vec![failure("github.com")]);
    }
}

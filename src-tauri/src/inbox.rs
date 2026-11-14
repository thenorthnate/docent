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
    let watched = store.watched_repos().await?;
    if watched.is_empty() {
        return Ok(ReviewRequested::default());
    }
    // Host names, and GitHub owner and repo names, all ignore case.
    let is_watched = |host: &str, owner: &str, repo: &str| {
        watched.iter().any(|w| {
            w.host.eq_ignore_ascii_case(host)
                && w.owner.eq_ignore_ascii_case(owner)
                && w.name.eq_ignore_ascii_case(repo)
        })
    };
    let mut found = forge.review_requested().await;
    found
        .pulls
        .retain(|pull| is_watched(&pull.host, &pull.owner, &pull.repo));
    // RFC 3339 timestamps in UTC sort as text.
    found.pulls.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    // A host with nothing watched on it has nothing to miss.
    found.failures.retain(|failure| {
        watched
            .iter()
            .any(|w| w.host.eq_ignore_ascii_case(&failure.host))
    });
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use docent_forge::{ConnectError, FakeForge, HostFailure, PullSummary, Section};
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
        FakeForge::with_pulls(SectionPulls { pulls, failures })
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

        let found = section(&forge, &store, Section::ReviewRequested).await.unwrap();

        let numbers: Vec<u64> = found.pulls.iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![3, 1]);
    }

    #[tokio::test]
    async fn watched_repos_on_several_hosts_are_listed_together() {
        let store = watching(&[
            ("github.com", "octocat/Hello-World"),
            ("GHE.corp.test", "mona/tools"),
        ])
        .await;
        let forge = forge(
            vec![
                pull(
                    "github.com",
                    "octocat/Hello-World",
                    1,
                    "2026-11-01T00:00:00Z",
                ),
                pull("ghe.corp.test", "mona/tools", 2, "2026-11-02T00:00:00Z"),
            ],
            vec![],
        );

        let found = section(&forge, &store, Section::ReviewRequested).await.unwrap();

        let numbers: Vec<u64> = found.pulls.iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![2, 1]);
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

        let found = section(&forge, &store, Section::ReviewRequested).await.unwrap();

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

        let found = section(&forge, &store, Section::ReviewRequested).await.unwrap();

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

        let found = section(&forge, &store, Section::ReviewRequested).await.unwrap();

        assert_eq!(found, SectionPulls::default());
        assert_eq!(forge.asked(), vec![]);
    }

    #[tokio::test]
    async fn only_failures_on_hosts_with_watched_repos_are_shown() {
        let store = watching(&[("github.com", "octocat/Hello-World")]).await;
        let forge = forge(
            vec![],
            vec![failure("github.com"), failure("ghe.corp.test")],
        );

        let found = section(&forge, &store, Section::ReviewRequested).await.unwrap();

        assert_eq!(found.failures, vec![failure("github.com")]);
    }

    #[tokio::test]
    async fn every_section_asks_for_itself_and_keeps_to_watched_repos() {
        let store = watching(&[("github.com", "octocat/Hello-World")]).await;
        for asked in [
            Section::ReviewRequested,
            Section::Assigned,
            Section::Authored,
            Section::Mentioned,
        ] {
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
                ],
                vec![],
            );

            let found = section(&forge, &store, asked).await.unwrap();

            assert_eq!(forge.asked(), vec![asked]);
            let numbers: Vec<u64> = found.pulls.iter().map(|p| p.number).collect();
            assert_eq!(numbers, vec![1], "{asked:?}");
        }
    }
}

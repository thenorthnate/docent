//! The review page: a PR's overview, for watched repos only.

use docent_forge::github::RepoName;
use docent_forge::{Forge, PullOverview};
use docent_store::Store;

use crate::command_error::CommandError;

/// The overview of PR `number` in `owner/repo` on `host`. The names come
/// from the page's URL, so they're parsed like typed input, and a repo that
/// isn't watched is refused before the forge is asked, as in the inbox.
pub async fn pull_overview(
    forge: &impl Forge,
    store: &Store,
    host: &str,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<PullOverview, CommandError> {
    let name = RepoName::parse(&format!("{owner}/{repo}"))?;
    let watched = store.watched_repos().await?;
    // Host names, and GitHub owner and repo names, all ignore case.
    let is_watched = watched.iter().any(|w| {
        w.host.eq_ignore_ascii_case(host)
            && w.owner.eq_ignore_ascii_case(name.owner())
            && w.name.eq_ignore_ascii_case(name.name())
    });
    if !is_watched {
        return Err(CommandError {
            kind: "not_watched",
            message: format!(
                "{}/{} on {host} isn't watched. Watch it in Settings → Repos to review its pull requests.",
                name.owner(),
                name.name()
            ),
        });
    }
    Ok(forge.pull_overview(host, &name, number).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use docent_forge::{CiStatus, FakeForge};
    use docent_store::NewRepo;

    async fn watching(host: &str, owner: &str, name: &str) -> Store {
        let store = Store::open_in_memory().await.expect("store");
        let account = store.upsert_account(host, "octocat").await.unwrap();
        let repo = NewRepo {
            owner: owner.to_string(),
            name: name.to_string(),
            default_branch: "main".to_string(),
        };
        store.watch_repo(account, &repo).await.unwrap();
        store
    }

    fn overview() -> PullOverview {
        PullOverview {
            title: "Fix the greeting".to_string(),
            author: "mona".to_string(),
            body: None,
            ci: CiStatus::Passing,
            checks: vec![],
            timeline: vec![],
            timeline_truncated: false,
        }
    }

    #[tokio::test]
    async fn a_watched_repos_pr_is_fetched_from_the_forge() {
        let store = watching("github.com", "octocat", "Hello-World").await;
        let forge = FakeForge::with_overview(overview());

        let found = pull_overview(&forge, &store, "github.com", "OctoCat", "hello-world", 1347)
            .await
            .unwrap();

        assert_eq!(found, overview());
        assert_eq!(
            forge.overviews_asked(),
            vec!["github.com/OctoCat/hello-world#1347"]
        );
    }

    #[tokio::test]
    async fn an_unwatched_repo_is_refused_without_asking_the_forge() {
        let store = watching("github.com", "octocat", "Hello-World").await;
        let forge = FakeForge::with_overview(overview());

        for (host, owner, repo) in [
            ("github.com", "octocat", "Spoon-Knife"),
            ("ghe.corp.test", "octocat", "Hello-World"),
        ] {
            let err = pull_overview(&forge, &store, host, owner, repo, 1347)
                .await
                .unwrap_err();

            assert_eq!(err.kind, "not_watched", "{host}/{owner}/{repo}");
        }
        assert_eq!(forge.overviews_asked(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn a_name_that_is_not_a_plain_repo_is_refused() {
        let store = watching("github.com", "octocat", "Hello-World").await;
        let forge = FakeForge::with_overview(overview());

        let err = pull_overview(&forge, &store, "github.com", "..", "Hello-World", 1)
            .await
            .unwrap_err();

        assert_eq!(err.kind, "invalid_name");
        assert_eq!(forge.overviews_asked(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn a_pr_the_forge_cannot_find_says_so() {
        let store = watching("github.com", "octocat", "Hello-World").await;
        let forge = FakeForge::default();

        let err = pull_overview(&forge, &store, "github.com", "octocat", "Hello-World", 9)
            .await
            .unwrap_err();

        assert_eq!(err.kind, "pull_not_found");
    }
}

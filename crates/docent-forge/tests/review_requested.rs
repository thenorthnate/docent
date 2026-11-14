//! The Review requested section, against a mock GitHub search API.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for};
use docent_forge::{Accounts, ConnectError, Forge, HostFailure, PullSummary, Section};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEARCH_JSON: &str = include_str!("fixtures/github-search-review-requested.json");

/// `GET /search/issues` for review-requested PRs, answering only `token`.
async fn search_answers(server: &MockServer, token: &str, body: &str) {
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param(
            "q",
            "is:pr is:open archived:false review-requested:@me",
        ))
        .and(query_param("per_page", "100"))
        .and(header("authorization", format!("Bearer {token}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.to_string(), "application/json"))
        .mount(server)
        .await;
}

fn pull(
    host: &str,
    repo: &str,
    number: u64,
    title: &str,
    author: &str,
    created_at: &str,
    body: Option<&str>,
) -> PullSummary {
    let (owner, name) = repo.split_once('/').expect("owner/name");
    PullSummary {
        host: host.to_string(),
        owner: owner.to_string(),
        repo: name.to_string(),
        number,
        title: title.to_string(),
        author: author.to_string(),
        created_at: created_at.to_string(),
        body: body.map(str::to_string),
    }
}

#[tokio::test]
async fn review_requested_prs_come_back_with_title_repo_author_and_age() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = github_for("gho_secret_a", "octocat").await;
    search_answers(&api, "gho_secret_a", SEARCH_JSON).await;
    let accounts = accounts_against(vec![("github.com", &api)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.failures, vec![]);
    assert_eq!(
        found.pulls,
        vec![
            pull(
                "github.com",
                "octocat/Hello-World",
                1347,
                "Fix the greeting",
                "mona",
                "2026-11-10T09:00:00Z",
                Some("Say hello properly.\n\nFixes #12.")
            ),
            pull(
                "github.com",
                "octocat/Spoon-Knife",
                42,
                "Sharpen the knife",
                "hubot",
                "2026-11-01T12:00:00Z",
                None
            ),
        ],
        "the item whose repository_url isn't a plain owner/name is dropped"
    );
}

#[tokio::test]
async fn each_pr_comes_back_with_its_description_or_none() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = github_for("gho_secret_a", "octocat").await;
    search_answers(&api, "gho_secret_a", SEARCH_JSON).await;
    let accounts = accounts_against(vec![("github.com", &api)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();

    let found = accounts.search(Section::ReviewRequested).await;

    let bodies: Vec<_> = found.pulls.iter().map(|p| p.body.as_deref()).collect();
    assert_eq!(
        bodies,
        vec![Some("Say hello properly.\n\nFixes #12."), None],
        "a PR with no description has a null body"
    );
}

#[tokio::test]
async fn each_host_is_searched_with_its_own_token() {
    let gh = FakeGh::with_tokens(&[
        ("github.com", "gho_secret_a"),
        ("ghe.corp.test", "gho_secret_b"),
    ]);
    let public = github_for("gho_secret_a", "octocat").await;
    let enterprise = github_for("gho_secret_b", "mona").await;
    search_answers(&public, "gho_secret_a", r#"{"items":[]}"#).await;
    let one_pr = SEARCH_JSON.replace("api.github.com", "ghe.corp.test/api/v3");
    search_answers(&enterprise, "gho_secret_b", &one_pr).await;
    let accounts = accounts_against(vec![
        ("github.com", &public),
        ("ghe.corp.test", &enterprise),
    ]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts.connect("ghe.corp.test", &gh.cli()).await.unwrap();

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.failures, vec![]);
    assert_eq!(found.pulls.len(), 2);
    assert!(found.pulls.iter().all(|p| p.host == "ghe.corp.test"));
}

#[tokio::test]
async fn a_failing_host_is_reported_and_the_others_still_show() {
    let gh = FakeGh::with_tokens(&[
        ("github.com", "gho_secret_a"),
        ("ghe.corp.test", "gho_secret_b"),
    ]);
    let public = github_for("gho_secret_a", "octocat").await;
    search_answers(&public, "gho_secret_a", SEARCH_JSON).await;
    let enterprise = github_for("gho_secret_b", "mona").await;
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&enterprise)
        .await;
    let accounts = accounts_against(vec![
        ("github.com", &public),
        ("ghe.corp.test", &enterprise),
    ]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts.connect("ghe.corp.test", &gh.cli()).await.unwrap();

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.pulls.len(), 2);
    assert_eq!(
        found.failures,
        vec![HostFailure {
            host: "ghe.corp.test".to_string(),
            error: ConnectError::HttpStatus {
                host: "ghe.corp.test".to_string(),
                status: 503
            },
        }]
    );
}

#[tokio::test]
async fn a_host_that_failed_to_reconnect_is_reported_without_an_api_call() {
    let gh = FakeGh::with_tokens(&[]);
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without a token"));
    let _ = accounts.restore("github.com", "octocat", &gh.cli()).await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.pulls, vec![]);
    assert_eq!(
        found.failures,
        vec![HostFailure {
            host: "github.com".to_string(),
            error: ConnectError::NotLoggedIn("github.com".to_string()),
        }]
    );
}

#[tokio::test]
async fn no_connected_hosts_means_nothing_to_show() {
    let accounts = Accounts::with_api_base(|_| unreachable!("no hosts, no API call"));

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.pulls, vec![]);
    assert_eq!(found.failures, vec![]);
}

// ---- Denial test: the token must not leave through a search error. ----

#[tokio::test]
async fn search_errors_never_carry_the_token() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = github_for("gho_secret_a", "octocat").await;
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .respond_with(ResponseTemplate::new(422).set_body_string("echo: gho_secret_a"))
        .mount(&api)
        .await;
    let accounts = accounts_against(vec![("github.com", &api)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.failures.len(), 1);
    let shown = format!("{:?} {}", found.failures, found.failures[0].error);
    assert!(!shown.contains("gho_secret_a"), "token leaked: {shown}");
}

#[tokio::test]
async fn a_repo_owned_by_a_user_named_repos_is_kept() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = github_for("gho_secret_a", "octocat").await;
    let body = SEARCH_JSON.replace("repos/octocat/Hello-World", "repos/repos/Hello-World");
    search_answers(&api, "gho_secret_a", &body).await;
    let accounts = accounts_against(vec![("github.com", &api)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.pulls[0].owner, "repos");
    assert_eq!(found.pulls[0].repo, "Hello-World");
}

#[tokio::test]
async fn a_host_that_does_not_answer_times_out_and_the_others_still_show() {
    let gh = FakeGh::with_tokens(&[
        ("github.com", "gho_secret_a"),
        ("ghe.corp.test", "gho_secret_b"),
    ]);
    let public = github_for("gho_secret_a", "octocat").await;
    search_answers(&public, "gho_secret_a", SEARCH_JSON).await;
    let enterprise = github_for("gho_secret_b", "mona").await;
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(60)))
        .mount(&enterprise)
        .await;
    let accounts = accounts_against(vec![
        ("github.com", &public),
        ("ghe.corp.test", &enterprise),
    ])
    // The timeout covers the healthy host too, so it must be generous enough
    // for a slow CI runner; the silent host waits far longer than that.
    .with_request_timeout(std::time::Duration::from_secs(2));
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts.connect("ghe.corp.test", &gh.cli()).await.unwrap();

    let found = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        accounts.search(Section::ReviewRequested),
    )
    .await
    .expect("the slow host was given up on");

    assert_eq!(found.pulls.len(), 2);
    assert_eq!(found.failures.len(), 1);
    assert_eq!(found.failures[0].host, "ghe.corp.test");
    assert_eq!(found.failures[0].error.kind(), "unreachable");
}

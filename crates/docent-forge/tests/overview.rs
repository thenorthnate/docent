//! A PR's description, checks and timeline for the Overview tab, fetched
//! with one GraphQL query against a mock GitHub API.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for};
use docent_forge::github::RepoName;
use docent_forge::{
    Accounts, Check, CheckState, CiStatus, Forge, PullOverview, RepoError, ReviewVerdict,
    TimelineItem,
};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "gho_secret_a";
const OVERVIEW_JSON: &str = include_str!("fixtures/github-graphql-pull-overview.json");

/// `POST graphql_path` for octocat/Hello-World#1347 answers with `response`.
async fn overview_answer(server: &MockServer, graphql_path: &str, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(graphql_path))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .and(body_partial_json(serde_json::json!({
            "variables": { "owner": "octocat", "name": "Hello-World", "number": 1347 }
        })))
        .respond_with(response)
        .mount(server)
        .await;
}

fn ok(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.to_string(), "application/json")
}

fn hello_world() -> RepoName {
    RepoName::parse("octocat/Hello-World").unwrap()
}

async fn connected(server: &MockServer) -> Accounts {
    let gh = FakeGh::with_tokens(&[("github.com", TOKEN)]);
    let accounts = accounts_against(vec![("github.com", server)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts
}

async fn overview(server: &MockServer) -> Result<PullOverview, RepoError> {
    connected(server)
        .await
        .pull_overview("github.com", &hello_world(), 1347)
        .await
}

fn check(name: &str, state: CheckState) -> Check {
    Check {
        name: name.to_string(),
        state,
    }
}

#[tokio::test]
async fn the_overview_carries_description_checks_and_timeline() {
    let api = github_for(TOKEN, "octocat").await;
    overview_answer(&api, "/graphql", ok(OVERVIEW_JSON)).await;

    let found = overview(&api).await.unwrap();

    assert_eq!(
        found,
        PullOverview {
            title: "Fix the greeting".to_string(),
            author: "mona".to_string(),
            body: Some("Say hello **properly**.\n\nFixes #12.".to_string()),
            ci: CiStatus::Failing,
            checks: vec![
                check("build", CheckState::Passing),
                check("test", CheckState::Failing),
                check("lint", CheckState::Pending),
                check("deploy", CheckState::Skipped),
                check("ci/legacy", CheckState::Pending),
            ],
            timeline: vec![
                TimelineItem::Commit {
                    sha: "6dcb09b5b57875f334f61aebed695e2e4193db5e".to_string(),
                    headline: "Say hello properly".to_string(),
                    author: "mona".to_string(),
                    at: "2026-11-10T09:00:00Z".to_string(),
                },
                // No GitHub account matches: the git name stands in.
                TimelineItem::Commit {
                    sha: "c3d0be41ecbe669545ee3e94d31ed9a4bc91ee3c".to_string(),
                    headline: "Fix a typo".to_string(),
                    author: "Someone Else".to_string(),
                    at: "2026-11-10T10:00:00Z".to_string(),
                },
                TimelineItem::Comment {
                    author: "hubot".to_string(),
                    body: "Looks close.".to_string(),
                    at: "2026-11-11T08:00:00Z".to_string(),
                },
                TimelineItem::Review {
                    author: "octocat".to_string(),
                    verdict: ReviewVerdict::ChangesRequested,
                    body: Some("Please add a test.".to_string()),
                    at: "2026-11-11T09:00:00Z".to_string(),
                },
                // The pending draft is skipped; a deleted author is `ghost`.
                TimelineItem::Review {
                    author: "ghost".to_string(),
                    verdict: ReviewVerdict::Approved,
                    body: None,
                    at: "2026-11-12T09:00:00Z".to_string(),
                },
            ],
            timeline_truncated: false,
        }
    );
}

#[tokio::test]
async fn a_repo_or_pr_github_does_not_show_is_not_found() {
    for data in [
        serde_json::json!({ "repository": null }),
        serde_json::json!({ "repository": { "pullRequest": null } }),
    ] {
        let api = github_for(TOKEN, "octocat").await;
        let body = serde_json::json!({ "data": data, "errors": [{ "type": "NOT_FOUND" }] });
        overview_answer(&api, "/graphql", ok(&body.to_string())).await;

        let err = overview(&api).await.unwrap_err();

        assert_eq!(
            err,
            RepoError::PullNotFound {
                host: "github.com".to_string(),
                repo: "octocat/Hello-World".to_string(),
                number: 1347,
            },
            "{data}"
        );
        assert_eq!(err.kind(), "pull_not_found");
    }
}

#[tokio::test]
async fn a_failed_query_is_an_error_without_githubs_message() {
    let api = github_for(TOKEN, "octocat").await;
    let body = r#"{ "data": null, "errors": [{ "message": "<script>alert(1)</script>" }] }"#;
    overview_answer(&api, "/graphql", ok(body)).await;

    let err = overview(&api).await.unwrap_err();

    assert_eq!(err.kind(), "unreachable");
    assert!(!err.to_string().contains("script"), "{err}");
}

#[tokio::test]
async fn a_rejected_token_says_how_to_refresh_it() {
    let api = github_for(TOKEN, "octocat").await;
    overview_answer(&api, "/graphql", ResponseTemplate::new(401)).await;

    let err = overview(&api).await.unwrap_err();

    assert_eq!(err.kind(), "token_rejected");
}

#[tokio::test]
async fn a_host_that_is_not_connected_is_not_asked() {
    let api = github_for(TOKEN, "octocat").await;
    overview_answer(&api, "/graphql", ok(OVERVIEW_JSON)).await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let err = accounts
        .pull_overview("github.com", &hello_world(), 1347)
        .await
        .unwrap_err();

    assert_eq!(err, RepoError::NotConnected("github.com".to_string()));
    let asked_graphql = api
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .any(|request| request.url.path() == "/graphql");
    assert!(!asked_graphql);
}

#[tokio::test]
async fn an_enterprise_host_is_asked_at_its_api_graphql_path() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/user"))
        .respond_with(ok(common::USER_JSON))
        .mount(&api)
        .await;
    overview_answer(&api, "/api/graphql", ok(OVERVIEW_JSON)).await;
    let gh = FakeGh::with_tokens(&[("ghe.corp.test", TOKEN)]);
    let api_base = format!("{}/api/v3", api.uri());
    let accounts = Accounts::with_api_base(move |_| api_base.clone());
    accounts.connect("ghe.corp.test", &gh.cli()).await.unwrap();

    let found = accounts
        .pull_overview("ghe.corp.test", &hello_world(), 1347)
        .await
        .unwrap();

    assert_eq!(found.title, "Fix the greeting");
}

#[tokio::test]
async fn a_long_timeline_is_marked_truncated_and_unknown_events_are_skipped() {
    let api = github_for(TOKEN, "octocat").await;
    let body = serde_json::json!({ "data": { "repository": { "pullRequest": {
        "title": "Fix the greeting",
        "body": "",
        "author": null,
        "commits": { "nodes": [{ "commit": { "statusCheckRollup": null } }] },
        "timelineItems": {
            "pageInfo": { "hasPreviousPage": true },
            "nodes": [{ "__typename": "SomethingNew" }, {}],
        },
    } } } });
    overview_answer(&api, "/graphql", ok(&body.to_string())).await;

    let found = overview(&api).await.unwrap();

    assert_eq!(
        (found.author.as_str(), found.body, found.ci),
        ("ghost", None, CiStatus::NoChecks)
    );
    assert_eq!(found.checks, vec![]);
    assert_eq!(found.timeline, vec![]);
    assert!(found.timeline_truncated);
}

//! Whether PRs are open, closed or merged, asked in one GraphQL query per
//! host, against a mock GitHub API.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for};
use docent_forge::{Forge, PullRef, PullState};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "gho_secret_a";

fn pull(host: &str, repo: &str, number: u64) -> PullRef {
    let (owner, name) = repo.split_once('/').expect("owner/name");
    PullRef {
        host: host.to_string(),
        owner: owner.to_string(),
        repo: name.to_string(),
        number,
    }
}

fn ok(body: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

/// Each repo's owner and name go as variables, never in the query text.
async fn states_answer(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .and(body_partial_json(serde_json::json!({
            "variables": {
                "o0": "octocat", "n0": "Hello-World",
                "o1": "octocat", "n1": "Spoon-Knife",
            }
        })))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

async fn connected(server: &MockServer) -> docent_forge::Accounts {
    let gh = FakeGh::with_tokens(&[("github.com", TOKEN)]);
    let accounts = accounts_against(vec![("github.com", server)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts
}

fn asked() -> Vec<PullRef> {
    vec![
        pull("github.com", "octocat/Hello-World", 1),
        pull("github.com", "octocat/Hello-World", 2),
        pull("github.com", "octocat/Spoon-Knife", 3),
        pull("github.com", "octocat/Hello-World", 4),
        pull("github.com", "octocat/Hello-World", 5),
    ]
}

/// PR 4 is gone or hidden, and PR 5 is in a state Docent doesn't know, so
/// neither is reported.
#[tokio::test]
async fn every_host_is_asked_once_for_all_its_prs() {
    let api = github_for(TOKEN, "octocat").await;
    states_answer(
        &api,
        ok(serde_json::json!({
            "data": {
                "r0": {
                    "p1": { "state": "OPEN" },
                    "p2": { "state": "MERGED" },
                    "p4": null,
                    "p5": { "state": "SOMETHING_NEW" },
                },
                "r1": { "p3": { "state": "CLOSED" } },
            },
            "errors": [{ "message": "Could not resolve to a PullRequest with the number of 4." }],
        })),
    )
    .await;
    let accounts = connected(&api).await;

    let found = accounts.pull_states(&asked()).await;

    assert_eq!(found.failures, vec![]);
    assert_eq!(
        found.states,
        vec![
            (
                pull("github.com", "octocat/Hello-World", 1),
                PullState::Open
            ),
            (
                pull("github.com", "octocat/Hello-World", 2),
                PullState::Merged
            ),
            (
                pull("github.com", "octocat/Spoon-Knife", 3),
                PullState::Closed
            ),
        ]
    );
}

#[tokio::test]
async fn a_failed_query_reports_the_host_and_no_states() {
    let api = github_for(TOKEN, "octocat").await;
    states_answer(&api, ResponseTemplate::new(403)).await;
    let accounts = connected(&api).await;

    let found = accounts.pull_states(&asked()).await;

    assert_eq!(found.states, vec![]);
    assert_eq!(found.failures.len(), 1);
    assert_eq!(found.failures[0].host, "github.com");
}

#[tokio::test]
async fn a_query_that_returns_no_data_reports_the_host() {
    let api = github_for(TOKEN, "octocat").await;
    let errors =
        serde_json::json!({ "data": null, "errors": [{ "message": "Something went wrong" }] });
    states_answer(&api, ok(errors)).await;
    let accounts = connected(&api).await;

    let found = accounts.pull_states(&asked()).await;

    assert_eq!(found.states, vec![]);
    assert_eq!(found.failures.len(), 1);
}

/// A repo the answer doesn't mention is left unknown, not taken as closed.
#[tokio::test]
async fn a_repo_left_out_of_the_answer_is_unknown() {
    let api = github_for(TOKEN, "octocat").await;
    states_answer(
        &api,
        ok(serde_json::json!({ "data": { "r0": { "p1": { "state": "CLOSED" } } } })),
    )
    .await;
    let accounts = connected(&api).await;

    let found = accounts.pull_states(&asked()).await;

    let closed = (
        pull("github.com", "octocat/Hello-World", 1),
        PullState::Closed,
    );
    assert_eq!(found.states, vec![closed]);
}

#[tokio::test]
async fn a_host_that_is_not_connected_is_reported_and_not_asked() {
    let api = github_for(TOKEN, "octocat").await;
    let accounts = connected(&api).await;

    let found = accounts
        .pull_states(&[pull("ghe.corp.test", "octocat/Hello-World", 1)])
        .await;

    assert_eq!(found.states, vec![]);
    assert_eq!(found.failures.len(), 1);
    assert_eq!(found.failures[0].host, "ghe.corp.test");
}

#[tokio::test]
async fn asking_about_no_prs_asks_nothing() {
    let api = github_for(TOKEN, "octocat").await;
    let accounts = connected(&api).await;

    let found = accounts.pull_states(&[]).await;

    assert_eq!(found.states, vec![]);
    assert_eq!(found.failures, vec![]);
    let posts = api.received_requests().await.unwrap();
    assert!(posts.iter().all(|r| r.method.as_str() != "POST"));
}

//! Inbox rows' CI status, review state and size, fetched with GraphQL after
//! the search, against a mock GitHub API.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for};
use docent_forge::{CiStatus, Forge, PullDetails, ReviewState, Section};
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "gho_secret_a";
const SEARCH_JSON: &str = include_str!("fixtures/github-search-review-requested.json");
const DETAILS_JSON: &str = include_str!("fixtures/github-graphql-pull-details.json");

/// Review requested answers with `SEARCH_JSON` under `prefix`.
async fn search_answers(server: &MockServer, prefix: &str) {
    Mock::given(method("GET"))
        .and(path(format!("{prefix}/search/issues")))
        .and(query_param(
            "q",
            "is:pr is:open archived:false review-requested:@me",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_raw(SEARCH_JSON, "application/json"))
        .mount(server)
        .await;
}

/// `POST graphql_path` for the search's PRs answers with `response`. The
/// hostile row (#7) is dropped before the query, so its ID is never sent.
async fn details_answer(server: &MockServer, graphql_path: &str, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(graphql_path))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .and(body_partial_json(serde_json::json!({
            "variables": { "ids": ["PR_kwDOAAABHM4AAAFT", "PR_kwDOAAAB0c4AAAAq"] }
        })))
        .respond_with(response)
        .mount(server)
        .await;
}

fn ok(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.to_string(), "application/json")
}

async fn search(server: &MockServer) -> Vec<(u64, Option<PullDetails>)> {
    let gh = FakeGh::with_tokens(&[("github.com", TOKEN)]);
    let accounts = accounts_against(vec![("github.com", server)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    let found = accounts.search(Section::ReviewRequested).await;
    assert_eq!(found.failures, vec![]);
    found
        .pulls
        .into_iter()
        .map(|p| (p.number, p.details))
        .collect()
}

#[tokio::test]
async fn rows_carry_ci_status_review_state_and_size() {
    let api = github_for(TOKEN, "octocat").await;
    search_answers(&api, "").await;
    details_answer(&api, "/graphql", ok(DETAILS_JSON)).await;

    let rows = search(&api).await;

    assert_eq!(
        rows,
        vec![
            (
                1347,
                Some(PullDetails {
                    ci: CiStatus::Passing,
                    review: ReviewState::Approved,
                    additions: 120,
                    deletions: 30,
                    changed_files: 4,
                })
            ),
            (
                42,
                Some(PullDetails {
                    ci: CiStatus::NoChecks,
                    review: ReviewState::NoReviews,
                    additions: 3,
                    deletions: 0,
                    changed_files: 1,
                })
            ),
        ]
    );
}

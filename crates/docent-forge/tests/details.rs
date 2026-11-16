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

#[tokio::test]
async fn a_failed_details_query_still_shows_the_rows_without_details() {
    let api = github_for(TOKEN, "octocat").await;
    search_answers(&api, "").await;
    details_answer(&api, "/graphql", ResponseTemplate::new(502)).await;

    let rows = search(&api).await;

    assert_eq!(rows, vec![(1347, None), (42, None)]);
}

#[tokio::test]
async fn graphql_errors_without_data_leave_rows_without_details() {
    let api = github_for(TOKEN, "octocat").await;
    search_answers(&api, "").await;
    let errors = r#"{ "data": null, "errors": [{ "message": "Something went wrong" }] }"#;
    details_answer(&api, "/graphql", ok(errors)).await;

    let rows = search(&api).await;

    assert_eq!(rows, vec![(1347, None), (42, None)]);
}

#[tokio::test]
async fn an_enterprise_host_is_asked_at_its_api_graphql_path() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/user"))
        .respond_with(ok(common::USER_JSON))
        .mount(&api)
        .await;
    search_answers(&api, "/api/v3").await;
    details_answer(&api, "/api/graphql", ok(DETAILS_JSON)).await;
    let gh = FakeGh::with_tokens(&[("ghe.corp.test", TOKEN)]);
    let api_base = format!("{}/api/v3", api.uri());
    let accounts = docent_forge::Accounts::with_api_base(move |_| api_base.clone());
    accounts.connect("ghe.corp.test", &gh.cli()).await.unwrap();

    let found = accounts.search(Section::ReviewRequested).await;

    let with_details = found.pulls.iter().filter(|p| p.details.is_some()).count();
    assert_eq!(with_details, 2);
}

/// A PR node with these review and CI states and no changes.
fn node(id: &str, decision: Option<&str>, reviews: u64, ci: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "additions": 0,
        "deletions": 0,
        "changedFiles": 0,
        "reviewDecision": decision,
        "reviews": { "totalCount": reviews },
        "commits": { "nodes": [{ "commit": {
            "statusCheckRollup": ci.map(|state| serde_json::json!({ "state": state })),
        } }] },
    })
}

#[tokio::test]
async fn each_forge_state_maps_to_a_row_state() {
    let cases = [
        (
            Some("CHANGES_REQUESTED"),
            1,
            Some("FAILURE"),
            ReviewState::ChangesRequested,
            CiStatus::Failing,
        ),
        (
            Some("REVIEW_REQUIRED"),
            0,
            Some("ERROR"),
            ReviewState::ReviewRequired,
            CiStatus::Failing,
        ),
        (
            None,
            3,
            Some("PENDING"),
            ReviewState::Reviewed,
            CiStatus::Pending,
        ),
        (
            None,
            0,
            Some("EXPECTED"),
            ReviewState::NoReviews,
            CiStatus::Pending,
        ),
        (
            Some("SOMETHING_NEW"),
            0,
            Some("SOMETHING_NEW"),
            ReviewState::NoReviews,
            CiStatus::Pending,
        ),
    ];
    for (decision, reviews, ci, review_state, ci_status) in cases {
        let api = github_for(TOKEN, "octocat").await;
        search_answers(&api, "").await;
        let body = serde_json::json!({ "data": { "nodes": [
            node("PR_kwDOAAABHM4AAAFT", decision, reviews, ci),
            // GitHub couldn't resolve #42's ID.
            null,
        ] } });
        details_answer(&api, "/graphql", ok(&body.to_string())).await;

        let rows = search(&api).await;

        let details = rows[0].1.as_ref().expect("details for #1347");
        assert_eq!(
            (details.review, details.ci),
            (review_state, ci_status),
            "{decision:?} {ci:?}"
        );
        assert_eq!(rows[1], (42, None));
    }
}

#[tokio::test]
async fn nodes_docent_did_not_ask_for_are_ignored() {
    let api = github_for(TOKEN, "octocat").await;
    search_answers(&api, "").await;
    let body = serde_json::json!({ "data": { "nodes": [
        node("PR_somebody_else", Some("APPROVED"), 1, Some("SUCCESS")),
        // Not a PR: an issue's ID, say.
        { "id": "I_kwDOAAABHM4AAAFT" },
    ] } });
    details_answer(&api, "/graphql", ok(&body.to_string())).await;

    let rows = search(&api).await;

    assert_eq!(rows, vec![(1347, None), (42, None)]);
}

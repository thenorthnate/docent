//! The Assigned, Authored by me and Mentioned sections, against a mock
//! GitHub search API. Review requested has its own file.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for};
use docent_forge::{Forge, Section};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEARCH_JSON: &str = include_str!("fixtures/github-search-review-requested.json");

/// `GET /search/issues` for `query`, answering only `token`.
async fn search_answers(server: &MockServer, token: &str, query: &str, body: &str) {
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("q", query))
        .and(query_param("sort", "updated"))
        .and(query_param("order", "desc"))
        .and(query_param("per_page", "100"))
        .and(header("authorization", format!("Bearer {token}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.to_string(), "application/json"))
        .mount(server)
        .await;
}

#[tokio::test]
async fn each_section_sends_its_own_search() {
    for (section, query) in [
        (
            Section::ReviewRequested,
            "is:pr is:open archived:false review-requested:@me",
        ),
        (
            Section::Assigned,
            "is:pr is:open archived:false assignee:@me",
        ),
        (Section::Authored, "is:pr is:open archived:false author:@me"),
        (
            Section::Mentioned,
            "is:pr is:open archived:false mentions:@me",
        ),
    ] {
        let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
        let api = github_for("gho_secret_a", "octocat").await;
        // Only this section's search answers; any other query gets a 404.
        search_answers(&api, "gho_secret_a", query, SEARCH_JSON).await;
        let accounts = accounts_against(vec![("github.com", &api)]);
        accounts.connect("github.com", &gh.cli()).await.unwrap();

        let found = accounts.search(section).await;

        assert_eq!(found.failures, vec![], "{section:?}");
        let numbers: Vec<u64> = found.pulls.iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![1347, 42], "{section:?}");
    }
}

#[test]
fn sections_arrive_from_the_ui_by_snake_case_name() {
    let parsed: Vec<Section> =
        serde_json::from_str(r#"["review_requested", "assigned", "authored", "mentioned"]"#)
            .unwrap();

    assert_eq!(
        parsed,
        vec![
            Section::ReviewRequested,
            Section::Assigned,
            Section::Authored,
            Section::Mentioned,
        ]
    );
}

#[test]
fn an_unknown_section_name_is_rejected() {
    assert!(serde_json::from_str::<Section>(r#""pinned""#).is_err());
}

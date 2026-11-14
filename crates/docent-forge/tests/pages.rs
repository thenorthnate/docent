//! Inbox searches across several pages of results, against a mock GitHub
//! search API.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for};
use docent_forge::{Accounts, Forge, Section};
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const QUERY: &str = "is:pr is:open archived:false review-requested:@me";

/// A page of search results holding PRs `numbers`, all in octocat/Hello-World.
fn page_body(numbers: &[u64], total_count: u64, incomplete: bool) -> String {
    let items: Vec<serde_json::Value> = numbers
        .iter()
        .map(|number| {
            serde_json::json!({
                "repository_url": "https://api.github.com/repos/octocat/Hello-World",
                "number": number,
                "title": format!("PR {number}"),
                "user": { "login": "mona" },
                "created_at": "2026-11-10T09:00:00Z",
            })
        })
        .collect();
    serde_json::json!({
        "total_count": total_count,
        "incomplete_results": incomplete,
        "items": items,
    })
    .to_string()
}

/// The `Link` header GitHub sends when there is a page after this one.
fn next_link(base: &str, page: u32) -> String {
    format!(
        "<{base}/search/issues?q=is%3Apr+is%3Aopen+archived%3Afalse+review-requested%3A%40me\
         &sort=updated&order=desc&per_page=100&page={page}>; rel=\"next\""
    )
}

/// Answers the search's page `page` (`None` for the first, which has no
/// `page` parameter) with `body`, and a `next` link when given one. Only
/// `token` is answered.
async fn page_answers(
    server: &MockServer,
    token: &str,
    page: Option<u32>,
    body: String,
    next: Option<String>,
) {
    let mock = Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("q", QUERY))
        .and(header("authorization", format!("Bearer {token}").as_str()));
    let mock = match page {
        Some(page) => mock.and(query_param("page", page.to_string())),
        None => mock.and(query_param_is_missing("page")),
    };
    let mut response = ResponseTemplate::new(200).set_body_raw(body, "application/json");
    if let Some(next) = next {
        response = response.insert_header("link", next.as_str());
    }
    mock.respond_with(response).mount(server).await;
}

async fn connected(api: &MockServer) -> Accounts {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let accounts = accounts_against(vec![("github.com", api)]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts
}

#[tokio::test]
async fn every_page_is_fetched_and_joined_in_order() {
    let api = github_for("gho_secret_a", "octocat").await;
    let base = api.uri();
    page_answers(
        &api,
        "gho_secret_a",
        None,
        page_body(&[1, 2], 5, false),
        Some(next_link(&base, 2)),
    )
    .await;
    page_answers(
        &api,
        "gho_secret_a",
        Some(2),
        page_body(&[3, 4], 5, false),
        Some(next_link(&base, 3)),
    )
    .await;
    page_answers(
        &api,
        "gho_secret_a",
        Some(3),
        page_body(&[5], 5, false),
        None,
    )
    .await;
    let accounts = connected(&api).await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.failures, vec![]);
    let numbers: Vec<u64> = found.pulls.iter().map(|p| p.number).collect();
    assert_eq!(numbers, vec![1, 2, 3, 4, 5]);
    assert_eq!(found.incomplete, Vec::<String>::new());
}

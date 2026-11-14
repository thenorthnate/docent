//! Inbox searches across several pages of results, against a mock GitHub
//! search API.
#![cfg(unix)]

mod common;

use common::{FakeGh, github_for};
use docent_forge::{Accounts, Forge, Section, SectionPulls};
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "gho_secret_a";

/// A page of search results holding PRs `numbers`, all in octocat/Hello-World.
fn page(numbers: &[u64], total_count: u64, incomplete: bool) -> String {
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
    serde_json::json!({ "total_count": total_count, "incomplete_results": incomplete, "items": items })
        .to_string()
}

/// The `Link` header GitHub sends when page `page` of the search comes next.
fn next_link(api_base: &str, page: usize) -> String {
    format!(
        "<{api_base}/search/issues?q=is%3Apr+is%3Aopen+archived%3Afalse+review-requested%3A%40me\
         &sort=updated&order=desc&per_page=100&page={page}>; rel=\"next\""
    )
}

/// Serves `pages` as one search under `prefix` (the API base's path), each
/// linking to the next, and the last linking to `last_next` if given. Only
/// [`TOKEN`] is answered.
async fn serve_chain(
    server: &MockServer,
    prefix: &str,
    pages: Vec<String>,
    last_next: Option<String>,
) {
    let api_base = format!("{}{prefix}", server.uri());
    let count = pages.len();
    for (i, body) in pages.into_iter().enumerate() {
        let number = i + 1;
        let mock = Mock::given(method("GET"))
            .and(path(format!("{prefix}/search/issues")))
            .and(query_param(
                "q",
                "is:pr is:open archived:false review-requested:@me",
            ))
            .and(header("authorization", format!("Bearer {TOKEN}").as_str()));
        let mock = match number {
            1 => mock.and(query_param_is_missing("page")),
            n => mock.and(query_param("page", n.to_string())),
        };
        let next = if number < count {
            Some(next_link(&api_base, number + 1))
        } else {
            last_next.clone()
        };
        let mut response = ResponseTemplate::new(200).set_body_raw(body, "application/json");
        if let Some(next) = next {
            response = response.insert_header("link", next.as_str());
        }
        mock.respond_with(response).mount(server).await;
    }
}

/// Accounts connected to github.com, whose API is `server` under `prefix`.
async fn connected(server: &MockServer, prefix: &str) -> Accounts {
    let gh = FakeGh::with_tokens(&[("github.com", TOKEN)]);
    let api_base = format!("{}{prefix}", server.uri());
    let accounts = Accounts::with_api_base(move |_| api_base.clone());
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts
}

fn numbers(found: &SectionPulls) -> Vec<u64> {
    found.pulls.iter().map(|p| p.number).collect()
}

fn flagged() -> Vec<String> {
    vec!["github.com".to_string()]
}

#[tokio::test]
async fn every_page_is_fetched_and_joined_in_order() {
    let api = github_for(TOKEN, "octocat").await;
    let pages = vec![
        page(&[1, 2], 5, false),
        page(&[3, 4], 5, false),
        page(&[5], 5, false),
    ];
    serve_chain(&api, "", pages, None).await;
    let accounts = connected(&api, "").await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.failures, vec![]);
    assert_eq!(numbers(&found), vec![1, 2, 3, 4, 5]);
    assert_eq!(found.incomplete, Vec::<String>::new());
}

#[tokio::test]
async fn pages_are_followed_under_an_enterprise_api_path() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/user"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(common::USER_JSON, "application/json"),
        )
        .mount(&api)
        .await;
    serve_chain(
        &api,
        "/api/v3",
        vec![page(&[1], 2, false), page(&[2], 2, false)],
        None,
    )
    .await;
    let accounts = connected(&api, "/api/v3").await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.failures, vec![]);
    assert_eq!(numbers(&found), vec![1, 2]);
}

#[tokio::test]
async fn results_github_says_are_incomplete_are_flagged() {
    let api = github_for(TOKEN, "octocat").await;
    serve_chain(&api, "", vec![page(&[1], 1, true)], None).await;
    let accounts = connected(&api, "").await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(
        numbers(&found),
        vec![1],
        "what did come back is still shown"
    );
    assert_eq!(found.incomplete, flagged());
}

#[tokio::test]
async fn more_results_than_github_returns_are_flagged() {
    let api = github_for(TOKEN, "octocat").await;
    // GitHub stops sending `next` at 1,000 results but still counts them all.
    serve_chain(&api, "", vec![page(&[1, 2], 1_500, false)], None).await;
    let accounts = connected(&api, "").await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.incomplete, flagged());
}

#[tokio::test]
async fn a_server_that_never_stops_paging_is_cut_off_at_ten_pages() {
    let api = github_for(TOKEN, "octocat").await;
    let pages = (1..=20).map(|n| page(&[n], 1, false)).collect();
    serve_chain(&api, "", pages, None).await;
    let accounts = connected(&api, "").await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(numbers(&found), (1..=10).collect::<Vec<u64>>());
    assert_eq!(found.incomplete, flagged());
}

#[tokio::test]
async fn a_later_page_that_fails_keeps_the_earlier_ones() {
    let api = github_for(TOKEN, "octocat").await;
    // Page 2 has no mock, so it gets a 404.
    let next = next_link(&api.uri(), 2);
    serve_chain(&api, "", vec![page(&[1], 2, false)], Some(next)).await;
    let accounts = connected(&api, "").await;

    let found = accounts.search(Section::ReviewRequested).await;

    assert_eq!(found.failures, vec![]);
    assert_eq!(numbers(&found), vec![1]);
    assert_eq!(found.incomplete, flagged());
}

// ---- Denial tests: the token must not follow a `next` link elsewhere. ----

#[tokio::test]
async fn the_token_does_not_follow_a_next_link_to_another_host() {
    let elsewhere = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(page(&[99], 1, false), "application/json"),
        )
        .mount(&elsewhere)
        .await;
    let api = github_for(TOKEN, "octocat").await;
    let next = next_link(&elsewhere.uri(), 2);
    serve_chain(&api, "", vec![page(&[1], 2, false)], Some(next)).await;
    let accounts = connected(&api, "").await;

    let found = accounts.search(Section::ReviewRequested).await;

    let received = elsewhere.received_requests().await.expect("recording on");
    assert!(
        received
            .iter()
            .all(|r| !r.headers.contains_key("authorization")),
        "token followed the next link: {:?}",
        received.iter().map(|r| &r.headers).collect::<Vec<_>>()
    );
    assert_eq!(
        numbers(&found),
        vec![1],
        "nothing from the other host is shown"
    );
    assert_eq!(found.incomplete, flagged());
}

#[tokio::test]
async fn a_next_link_to_another_endpoint_on_the_same_host_is_not_followed() {
    let api = github_for(TOKEN, "octocat").await;
    // `/user` answers with the token; a search page must not lead there.
    let next = format!("<{}/user?page=2>; rel=\"next\"", api.uri());
    serve_chain(&api, "", vec![page(&[1], 2, false)], Some(next)).await;
    let accounts = connected(&api, "").await;

    let found = accounts.search(Section::ReviewRequested).await;

    let received = api.received_requests().await.expect("recording on");
    let user_calls = received.iter().filter(|r| r.url.path() == "/user").count();
    assert_eq!(user_calls, 1, "only the connect asks for /user");
    assert_eq!(found.incomplete, flagged());
}

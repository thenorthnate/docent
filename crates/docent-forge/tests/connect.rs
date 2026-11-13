//! Connecting hosts end to end: a fake `gh` on a temp `PATH` supplies the
//! token, and a `wiremock` server stands in for the GitHub API.
#![cfg(unix)]

mod common;

use common::{FakeGh, USER_JSON, accounts_against, github_for, summary};
use docent_forge::{Accounts, ConnectError, GhCli};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn connecting_a_host_shows_it_with_the_users_login() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = github_for("gho_secret_a", "octocat").await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let connected = accounts.connect("github.com", &gh.cli()).await;

    assert_eq!(connected, Ok(summary("github.com", "octocat")));
    assert_eq!(accounts.list(), vec![summary("github.com", "octocat")]);
    assert_eq!(gh.invocations(), vec!["auth token --hostname github.com"]);
}

#[tokio::test]
async fn two_hosts_stay_connected_together_each_with_its_own_token() {
    let gh = FakeGh::with_tokens(&[
        ("github.com", "gho_secret_a"),
        ("ghe.corp.test", "gho_secret_b"),
    ]);
    // Each server only accepts its own host's token, so a token sent to the
    // wrong host fails the connect.
    let public = github_for("gho_secret_a", "octocat").await;
    let enterprise = github_for("gho_secret_b", "mona").await;
    let accounts = accounts_against(vec![
        ("github.com", &public),
        ("ghe.corp.test", &enterprise),
    ]);

    accounts
        .connect("github.com", &gh.cli())
        .await
        .expect("github.com");
    accounts
        .connect("GHE.corp.test", &gh.cli())
        .await
        .expect("enterprise");

    assert_eq!(
        accounts.list(),
        vec![
            summary("ghe.corp.test", "mona"),
            summary("github.com", "octocat")
        ]
    );
}

#[tokio::test]
async fn missing_gh_says_to_install_it() {
    let empty_path = tempfile::tempdir().expect("temp dir");
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without a token"));

    let err = accounts
        .connect("github.com", &GhCli::new(empty_path.path().into()))
        .await
        .unwrap_err();

    assert_eq!(err, ConnectError::GhNotFound);
    assert!(err.to_string().contains("Install the GitHub CLI"), "{err}");
    assert!(accounts.list().is_empty());
}

#[tokio::test]
async fn gh_not_logged_in_says_how_to_log_in() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without a token"));

    let err = accounts
        .connect("ghe.corp.test", &gh.cli())
        .await
        .unwrap_err();

    assert_eq!(err, ConnectError::NotLoggedIn("ghe.corp.test".to_string()));
    assert!(
        err.to_string()
            .contains("gh auth login --hostname ghe.corp.test"),
        "{err}"
    );
    assert!(accounts.list().is_empty());
}

#[tokio::test]
async fn a_rejected_token_says_to_refresh_the_gh_login() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_stale")]);
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(
            r#"{"message":"Bad credentials","documentation_url":"https://docs.github.com/rest"}"#,
            "application/json",
        ))
        .mount(&api)
        .await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let err = accounts.connect("github.com", &gh.cli()).await.unwrap_err();

    assert_eq!(err, ConnectError::TokenRejected("github.com".to_string()));
    assert!(
        err.to_string()
            .contains("gh auth refresh --hostname github.com"),
        "{err}"
    );
    assert!(accounts.list().is_empty());
}

// ---- Denial tests: the token must not leave the in-memory account. ----

#[tokio::test]
async fn hostile_hosts_never_reach_gh() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call for a bad host"));

    for hostile in [
        "--help",
        "-h github.com",
        "github.com --version",
        "https://evil.test",
    ] {
        let err = accounts.connect(hostile, &gh.cli()).await.unwrap_err();
        assert_eq!(err, ConnectError::InvalidHost(hostile.to_string()));
    }

    assert!(
        gh.invocations().is_empty(),
        "gh ran: {:?}",
        gh.invocations()
    );
}

#[tokio::test]
async fn the_token_is_never_in_what_the_ui_or_logs_can_see() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = github_for("gho_secret_a", "octocat").await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let connected = accounts
        .connect("github.com", &gh.cli())
        .await
        .expect("connect");

    let visible = [
        serde_json::to_string(&connected).expect("serialize summary"),
        serde_json::to_string(&accounts.list()).expect("serialize list"),
        format!("{connected:?} {accounts:?} {:#?}", accounts.list()),
    ];
    for text in visible {
        assert!(!text.contains("gho_secret_a"), "token leaked: {text}");
    }
}

#[tokio::test]
async fn errors_never_carry_the_token() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_string("echo: gho_secret_a"))
        .mount(&api)
        .await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let err = accounts.connect("github.com", &gh.cli()).await.unwrap_err();

    let shown = format!(
        "{err} {err:?} {}",
        serde_json::to_string(&err).expect("serialize error")
    );
    assert!(!shown.contains("gho_secret_a"), "token leaked: {shown}");
}

#[tokio::test]
async fn the_token_does_not_follow_a_redirect_to_another_host() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let elsewhere = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(USER_JSON, "application/json"))
        .mount(&elsewhere)
        .await;
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/user", elsewhere.uri())),
        )
        .mount(&api)
        .await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let _ = accounts.connect("github.com", &gh.cli()).await;

    let received = elsewhere.received_requests().await.expect("recording on");
    assert!(!received.is_empty(), "the redirect was not followed");
    assert!(
        received
            .iter()
            .all(|r| !r.headers.contains_key("authorization")),
        "token followed the redirect: {:?}",
        received.iter().map(|r| &r.headers).collect::<Vec<_>>()
    );
}

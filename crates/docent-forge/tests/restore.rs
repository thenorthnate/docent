//! Reconnecting remembered hosts at startup: a fake `gh` supplies the token
//! again, and a `wiremock` server stands in for the GitHub API.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for, summary};
use docent_forge::github::RepoName;
use docent_forge::{AccountSummary, Accounts, ConnectError, RepoError};

fn failed(host: &str, login: &str, error: ConnectError) -> AccountSummary {
    AccountSummary {
        error: Some(error),
        ..summary(host, login)
    }
}

#[tokio::test]
async fn a_remembered_host_reconnects_with_a_fresh_token_from_gh() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = github_for("gho_secret_a", "octocat").await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let restored = accounts.restore("github.com", "octocat", &gh.cli()).await;

    assert_eq!(restored, Ok(summary("github.com", "octocat")));
    assert_eq!(accounts.list(), vec![summary("github.com", "octocat")]);
    assert_eq!(gh.invocations(), vec!["auth token --hostname github.com"]);
}

#[tokio::test]
async fn a_host_that_fails_to_reconnect_stays_listed_with_its_error() {
    let gh = FakeGh::with_tokens(&[]);
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without a token"));

    let err = accounts
        .restore("ghe.corp.test", "mona", &gh.cli())
        .await
        .unwrap_err();

    let not_logged_in = ConnectError::NotLoggedIn("ghe.corp.test".to_string());
    assert_eq!(err, not_logged_in);
    assert_eq!(
        accounts.list(),
        vec![failed("ghe.corp.test", "mona", not_logged_in)]
    );
}

#[tokio::test]
async fn a_host_that_failed_to_reconnect_cannot_be_used_for_lookups() {
    let gh = FakeGh::with_tokens(&[]);
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without a token"));
    let _ = accounts.restore("github.com", "octocat", &gh.cli()).await;

    let err = accounts
        .lookup_repo(
            "github.com",
            &RepoName::parse("octocat/Hello-World").unwrap(),
        )
        .await
        .unwrap_err();

    assert_eq!(err, RepoError::NotConnected("github.com".to_string()));
}

#[tokio::test]
async fn connecting_again_clears_a_failed_reconnect() {
    let api = github_for("gho_secret_a", "octocat").await;
    let accounts = accounts_against(vec![("github.com", &api)]);
    let logged_out = FakeGh::with_tokens(&[]);
    let _ = accounts
        .restore("github.com", "octocat", &logged_out.cli())
        .await;

    let logged_in = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    accounts
        .connect("github.com", &logged_in.cli())
        .await
        .expect("reconnect");

    assert_eq!(accounts.list(), vec![summary("github.com", "octocat")]);
}

// ---- Denial tests: a remembered host is untrusted input from disk. ----

#[tokio::test]
async fn a_tampered_saved_host_never_reaches_gh_and_is_not_listed() {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call for a bad host"));

    for hostile in ["--help", "github.com --version", "https://evil.test"] {
        let err = accounts
            .restore(hostile, "octocat", &gh.cli())
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::InvalidHost(hostile.to_string()));
    }

    assert!(
        gh.invocations().is_empty(),
        "gh ran: {:?}",
        gh.invocations()
    );
    assert_eq!(accounts.list(), vec![]);
}

#[tokio::test]
async fn a_failed_reconnect_never_shows_a_token() {
    // gh hands out a token the server rejects, and the rejection echoes it.
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    let api = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(401).set_body_string("gho_secret_a"))
        .mount(&api)
        .await;
    let accounts = accounts_against(vec![("github.com", &api)]);

    let _ = accounts.restore("github.com", "octocat", &gh.cli()).await;

    let shown = format!(
        "{} {:?}",
        serde_json::to_string(&accounts.list()).expect("serialize"),
        accounts
    );
    assert!(shown.contains("token_rejected"), "{shown}");
    assert!(!shown.contains("gho_secret_a"), "token leaked: {shown}");
}

#[tokio::test]
async fn a_reconnect_that_hangs_gives_up_and_is_listed_as_unreachable() {
    // gh never answers, as with a stuck credential helper. `sleep` is named
    // by its full path: gh runs with `PATH` set to the fake's folder alone, so
    // a bare `sleep` isn't found and gh would fail fast instead of hanging.
    let gh = FakeGh::with_script("exec /bin/sleep 30\n");
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without a token"))
        .with_restore_timeout(std::time::Duration::from_secs(2));

    let started = std::time::Instant::now();
    let err = accounts
        .restore("github.com", "octocat", &gh.cli())
        .await
        .unwrap_err();

    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "did not give up"
    );
    assert_eq!(err.kind(), "unreachable");
    assert_eq!(accounts.list()[0].error, Some(err));
}

#[tokio::test]
async fn a_failed_reconnect_shows_the_latest_error() {
    let accounts = Accounts::with_api_base(|_| "http://127.0.0.1:9".to_string());
    let logged_out = FakeGh::with_tokens(&[]);
    let _ = accounts
        .restore("github.com", "octocat", &logged_out.cli())
        .await;

    // Reconnecting fails differently: gh is now missing altogether.
    let empty_path = tempfile::tempdir().expect("temp dir");
    let err = accounts
        .connect(
            "github.com",
            &docent_forge::GhCli::new(empty_path.path().into()),
        )
        .await
        .unwrap_err();

    assert_eq!(err, ConnectError::GhNotFound);
    assert_eq!(
        accounts.list(),
        vec![failed("github.com", "octocat", ConnectError::GhNotFound)]
    );
}

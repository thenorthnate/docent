//! Connecting hosts end to end: a fake `gh` on a temp `PATH` supplies the
//! token, and a `wiremock` server stands in for the GitHub API.
#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;

use docent_forge::{AccountSummary, Accounts, ConnectError, GhCli};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const USER_JSON: &str = include_str!("fixtures/github-user.json");

/// A temp dir holding a fake `gh` that answers `auth token --hostname <host>`
/// from `tokens` and records its arguments in `args.log`.
struct FakeGh {
    dir: TempDir,
}

impl FakeGh {
    fn with_tokens(tokens: &[(&str, &str)]) -> Self {
        let cases: String = tokens
            .iter()
            .map(|(host, token)| format!("  {host}) printf '%s\\n' '{token}' ;;\n"))
            .collect();
        Self::with_script(&format!(
            "case \"$4\" in\n{cases}  *) printf 'no oauth token found for %s\\n' \"$4\" >&2; exit 1 ;;\nesac\n"
        ))
    }

    fn with_script(body: &str) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let log = dir.path().join("args.log");
        let script = format!(
            "#!/bin/sh\nprintf '%s ' \"$@\" >> '{}'\nprintf '\\n' >> '{}'\n{body}",
            log.display(),
            log.display()
        );
        let gh = dir.path().join("gh");
        std::fs::write(&gh, script).expect("write fake gh");
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        Self { dir }
    }

    fn cli(&self) -> GhCli {
        GhCli::new(OsString::from(self.dir.path()))
    }

    fn invocations(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.path().join("args.log"))
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }
}

/// A GitHub API that answers `GET /user` only for `token`.
async fn github_for(token: &str, login: &str) -> MockServer {
    let server = MockServer::start().await;
    let body = USER_JSON.replace("octocat", login);
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("authorization", format!("Bearer {token}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;
    server
}

/// Accounts whose API base for each host is the matching mock server.
fn accounts_against(servers: Vec<(&str, &MockServer)>) -> Accounts {
    let bases: Vec<(String, String)> = servers
        .into_iter()
        .map(|(host, server)| (host.to_string(), server.uri()))
        .collect();
    Accounts::with_api_base(move |host| {
        bases
            .iter()
            .find(|(h, _)| h == host.as_str())
            .map(|(_, base)| base.clone())
            .unwrap_or_else(|| "http://127.0.0.1:9".to_string())
    })
}

fn summary(host: &str, login: &str) -> AccountSummary {
    AccountSummary {
        host: host.to_string(),
        login: login.to_string(),
    }
}

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

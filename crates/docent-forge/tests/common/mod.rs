//! Test doubles shared by the integration tests: a fake `gh` on a temp
//! `PATH`, and `wiremock` servers standing in for the GitHub API.
#![allow(dead_code)] // Each test file uses a different subset.

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;

use docent_forge::{AccountSummary, Accounts, GhCli};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const USER_JSON: &str = include_str!("../fixtures/github-user.json");

/// A temp dir holding a fake `gh` that answers `auth token --hostname <host>`
/// from `tokens` and records its arguments in `args.log`.
pub struct FakeGh {
    dir: TempDir,
}

impl FakeGh {
    pub fn with_tokens(tokens: &[(&str, &str)]) -> Self {
        let cases: String = tokens
            .iter()
            .map(|(host, token)| format!("  {host}) printf '%s\\n' '{token}' ;;\n"))
            .collect();
        Self::with_script(&format!(
            "case \"$4\" in\n{cases}  *) printf 'no oauth token found for %s\\n' \"$4\" >&2; exit 1 ;;\nesac\n"
        ))
    }

    pub fn with_script(body: &str) -> Self {
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

    pub fn cli(&self) -> GhCli {
        GhCli::new(OsString::from(self.dir.path()))
    }

    pub fn invocations(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.path().join("args.log"))
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }
}

/// A GitHub API that answers `GET /user` only for `token`.
pub async fn github_for(token: &str, login: &str) -> MockServer {
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
pub fn accounts_against(servers: Vec<(&str, &MockServer)>) -> Accounts {
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

pub fn summary(host: &str, login: &str) -> AccountSummary {
    AccountSummary {
        host: host.to_string(),
        login: login.to_string(),
        error: None,
    }
}

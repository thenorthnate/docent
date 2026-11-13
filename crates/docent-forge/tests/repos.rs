//! Looking up a repo before it is watched, against a mock GitHub API.
#![cfg(unix)]

mod common;

use common::{FakeGh, accounts_against, github_for, summary};
use docent_forge::{Accounts, ConnectError, ForgeRepo, RepoError, RepoName};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REPO_JSON: &str = include_str!("fixtures/github-repo.json");

/// A connected github.com account whose API is `api`, which also answers
/// `GET /user` for the token.
async fn connected(api: &MockServer) -> Accounts {
    let gh = FakeGh::with_tokens(&[("github.com", "gho_secret_a")]);
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(common::USER_JSON, "application/json"),
        )
        .mount(api)
        .await;
    let accounts = accounts_against(vec![("github.com", api)]);
    accounts
        .connect("github.com", &gh.cli())
        .await
        .expect("connect");
    accounts
}

fn name(input: &str) -> RepoName {
    RepoName::parse(input).expect("valid repo name")
}

#[tokio::test]
async fn an_existing_repo_comes_back_with_its_canonical_name_and_default_branch() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/OCTOCAT/hello-world"))
        .and(header("authorization", "Bearer gho_secret_a"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(REPO_JSON, "application/json"))
        .mount(&api)
        .await;
    let accounts = connected(&api).await;

    let found = accounts
        .lookup_repo("github.com", &name("OCTOCAT/hello-world"))
        .await;

    assert_eq!(
        found,
        Ok(ForgeRepo {
            account: summary("github.com", "octocat"),
            owner: "octocat".to_string(),
            name: "Hello-World".to_string(),
            default_branch: "master".to_string(),
        })
    );
}

#[tokio::test]
async fn a_missing_repo_says_it_was_not_found() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/nope"))
        .respond_with(ResponseTemplate::new(404).set_body_raw(
            r#"{"message":"Not Found","documentation_url":"https://docs.github.com/rest"}"#,
            "application/json",
        ))
        .mount(&api)
        .await;
    let accounts = connected(&api).await;

    let err = accounts
        .lookup_repo("github.com", &name("octocat/nope"))
        .await
        .unwrap_err();

    assert_eq!(
        err,
        RepoError::NotFound {
            host: "github.com".to_string(),
            repo: "octocat/nope".to_string()
        }
    );
    assert!(err.to_string().contains("can't see it"), "{err}");
}

#[tokio::test]
async fn a_rejected_token_says_to_refresh_the_gh_login() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&api)
        .await;
    let accounts = connected(&api).await;

    let err = accounts
        .lookup_repo("github.com", &name("octocat/Hello-World"))
        .await
        .unwrap_err();

    assert_eq!(
        err,
        RepoError::Connect(ConnectError::TokenRejected("github.com".to_string()))
    );
}

#[tokio::test]
async fn a_host_that_is_not_connected_makes_no_api_call() {
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without an account"));

    let err = accounts
        .lookup_repo("ghe.corp.test", &name("octocat/Hello-World"))
        .await
        .unwrap_err();

    assert_eq!(err, RepoError::NotConnected("ghe.corp.test".to_string()));
}

#[tokio::test]
async fn each_host_is_asked_with_its_own_token() {
    let gh = FakeGh::with_tokens(&[
        ("github.com", "gho_secret_a"),
        ("ghe.corp.test", "gho_secret_b"),
    ]);
    let public = github_for("gho_secret_a", "octocat").await;
    let enterprise = github_for("gho_secret_b", "mona").await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World"))
        .and(header("authorization", "Bearer gho_secret_b"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(REPO_JSON, "application/json"))
        .mount(&enterprise)
        .await;
    let accounts = accounts_against(vec![
        ("github.com", &public),
        ("ghe.corp.test", &enterprise),
    ]);
    accounts.connect("github.com", &gh.cli()).await.unwrap();
    accounts.connect("ghe.corp.test", &gh.cli()).await.unwrap();

    let found = accounts
        .lookup_repo("GHE.corp.test", &name("octocat/Hello-World"))
        .await
        .expect("found on the enterprise host");

    assert_eq!(found.account, summary("ghe.corp.test", "mona"));
}

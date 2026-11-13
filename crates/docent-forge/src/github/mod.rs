//! GitHub: its naming rules and the REST calls Docent makes.

mod repo_name;

pub use repo_name::RepoName;

use octocrab::Octocrab;

use crate::{ConnectError, Host, Token};

#[derive(serde::Deserialize)]
struct User {
    login: String,
}

#[derive(serde::Deserialize)]
struct Owner {
    login: String,
}

/// The fields of `GET /repos/{owner}/{repo}` that Docent keeps.
#[derive(serde::Deserialize)]
pub(crate) struct Repo {
    owner: Owner,
    pub(crate) name: String,
    pub(crate) default_branch: String,
}

impl Repo {
    pub(crate) fn owner_login(&self) -> &str {
        &self.owner.login
    }
}

/// A failed API call: the repo isn't there, or anything a connect can fail with.
pub(crate) enum ApiError {
    NotFound,
    Connect(ConnectError),
}

/// A client for `host`'s API at `api_base`, sending `token`.
fn client(host: &Host, api_base: &str, token: &Token) -> Result<Octocrab, ConnectError> {
    let unreachable = |reason: String| ConnectError::Unreachable {
        host: host.as_str().to_string(),
        reason,
    };
    Octocrab::builder()
        .base_uri(api_base)
        .map_err(|err| unreachable(err.to_string()))?
        .personal_token(token.expose().to_string())
        .build()
        .map_err(|err| unreachable(err.to_string()))
}

/// `GET path`, with errors mapped so none carries the response body.
async fn get<T: serde::de::DeserializeOwned>(
    host: &Host,
    api_base: &str,
    token: &Token,
    path: &str,
) -> Result<T, ApiError> {
    let client = client(host, api_base, token).map_err(ApiError::Connect)?;
    let host_name = || host.as_str().to_string();
    match client.get::<T, _, ()>(path, None).await {
        Ok(body) => Ok(body),
        // GitHub errors carry the response body, which the server controls,
        // so only the status is passed on.
        Err(octocrab::Error::GitHub { source, .. }) => match source.status_code.as_u16() {
            401 => Err(ApiError::Connect(ConnectError::TokenRejected(host_name()))),
            404 => Err(ApiError::NotFound),
            status => Err(ApiError::Connect(ConnectError::HttpStatus {
                host: host_name(),
                status,
            })),
        },
        Err(err) => Err(ApiError::Connect(ConnectError::Unreachable {
            host: host_name(),
            reason: err.to_string(),
        })),
    }
}

/// Fetches the login of the user `token` belongs to, with `GET /user`.
pub(crate) async fn current_login(
    host: &Host,
    api_base: &str,
    token: &Token,
) -> Result<String, ConnectError> {
    match get::<User>(host, api_base, token, "/user").await {
        Ok(user) => Ok(user.login),
        Err(ApiError::Connect(err)) => Err(err),
        Err(ApiError::NotFound) => Err(ConnectError::HttpStatus {
            host: host.as_str().to_string(),
            status: 404,
        }),
    }
}

/// Fetches a repo with `GET /repos/{owner}/{name}`. `RepoName` only holds
/// plain path segments, so the path can't be steered elsewhere.
pub(crate) async fn repo(
    host: &Host,
    api_base: &str,
    token: &Token,
    name: &RepoName,
) -> Result<Repo, ApiError> {
    let path = format!("/repos/{}/{}", name.owner(), name.name());
    get(host, api_base, token, &path).await
}

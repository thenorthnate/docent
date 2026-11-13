//! GitHub: its naming rules and the REST calls Docent makes.

mod repo_name;

pub use repo_name::RepoName;

use octocrab::Octocrab;

use crate::{ConnectError, Host, PullSummary, Token};

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

impl ApiError {
    /// For calls where a 404 has no meaning of its own.
    fn into_connect(self, host: &Host) -> ConnectError {
        match self {
            Self::Connect(err) => err,
            Self::NotFound => ConnectError::HttpStatus {
                host: host.as_str().to_string(),
                status: 404,
            },
        }
    }
}

/// Fetches the login of the user `token` belongs to, with `GET /user`.
pub(crate) async fn current_login(
    host: &Host,
    api_base: &str,
    token: &Token,
) -> Result<String, ConnectError> {
    get::<User>(host, api_base, token, "/user")
        .await
        .map(|user| user.login)
        .map_err(|err| err.into_connect(host))
}

/// The search for open PRs that request the user's review, directly or
/// through a team, newest update first. URL-encoded by hand because it never
/// changes: `is:pr is:open archived:false review-requested:@me`.
const REVIEW_REQUESTED_SEARCH: &str = "/search/issues\
    ?q=is%3Apr+is%3Aopen+archived%3Afalse+review-requested%3A%40me\
    &sort=updated&order=desc&per_page=100";

#[derive(serde::Deserialize)]
struct SearchResults {
    items: Vec<SearchItem>,
}

/// The fields of a search result that an inbox row needs.
#[derive(serde::Deserialize)]
struct SearchItem {
    repository_url: String,
    number: u64,
    title: String,
    user: User,
    created_at: String,
}

impl SearchItem {
    /// The row for this result, or `None` when `repository_url` doesn't end
    /// in a plain `/repos/{owner}/{name}`. The response is untrusted, so the
    /// repo goes through the same check as typed input.
    fn into_pull(self, host: &Host) -> Option<PullSummary> {
        let (_, repo) = self.repository_url.rsplit_once("/repos/")?;
        let repo = RepoName::parse(repo).ok()?;
        Some(PullSummary {
            host: host.as_str().to_string(),
            owner: repo.owner().to_string(),
            repo: repo.name().to_string(),
            number: self.number,
            title: self.title,
            author: self.user.login,
            created_at: self.created_at,
        })
    }
}

/// Open PRs on `host` that request the user's review. Only the first 100
/// are fetched.
pub(crate) async fn review_requested(
    host: &Host,
    api_base: &str,
    token: &Token,
) -> Result<Vec<PullSummary>, ConnectError> {
    let results: SearchResults = get(host, api_base, token, REVIEW_REQUESTED_SEARCH)
        .await
        .map_err(|err| err.into_connect(host))?;
    Ok(results
        .items
        .into_iter()
        .filter_map(|item| item.into_pull(host))
        .collect())
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

//! GitHub: its naming rules and the REST calls Docent makes.

mod details;
mod overview;
mod repo_name;
mod states;

pub(crate) use overview::pull_overview;
pub(crate) use states::pull_states;
pub use repo_name::RepoName;

use octocrab::{Octocrab, Page};
use tokio::time::Instant;

use crate::{CiStatus, ConnectError, Host, PullSummary, Section, Token};

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

/// Where `api_base`'s GraphQL endpoint lives, less the `/graphql`:
/// `https://api.github.com`, or `https://{host}/api` for GitHub Enterprise,
/// whose REST API is under `/api/v3`.
pub(crate) fn graphql_base(api_base: &str) -> &str {
    let base = api_base.trim_end_matches('/');
    base.strip_suffix("/v3").unwrap_or(base)
}

/// A head commit's CI status from its `statusCheckRollup.state`, or `None`
/// when it has no checks or statuses.
pub(crate) fn ci_status(rollup_state: Option<&str>) -> CiStatus {
    match rollup_state {
        None => CiStatus::NoChecks,
        Some("SUCCESS") => CiStatus::Passing,
        Some("FAILURE" | "ERROR") => CiStatus::Failing,
        // `PENDING`, `EXPECTED`, and any state GitHub adds later: not settled.
        Some(_) => CiStatus::Pending,
    }
}

/// `GET path`, with errors mapped so none carries the response body.
async fn get<T: octocrab::FromResponse>(
    host: &Host,
    api_base: &str,
    token: &Token,
    path: &str,
) -> Result<T, ApiError> {
    let client = client(host, api_base, token).map_err(ApiError::Connect)?;
    client
        .get::<T, _, ()>(path, None)
        .await
        .map_err(|err| api_error(host, err))
}

/// `POST path` with a JSON `body`, with errors mapped as for [`get`].
async fn post<T: octocrab::FromResponse>(
    host: &Host,
    api_base: &str,
    token: &Token,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<T, ApiError> {
    let client = client(host, api_base, token).map_err(ApiError::Connect)?;
    client
        .post(path, Some(body))
        .await
        .map_err(|err| api_error(host, err))
}

/// GitHub errors carry the response body, which the server controls, so
/// only the status is passed on.
fn api_error(host: &Host, err: octocrab::Error) -> ApiError {
    let host_name = || host.as_str().to_string();
    match err {
        octocrab::Error::GitHub { source, .. } => match source.status_code.as_u16() {
            401 => ApiError::Connect(ConnectError::TokenRejected(host_name())),
            404 => ApiError::NotFound,
            status => ApiError::Connect(ConnectError::HttpStatus {
                host: host_name(),
                status,
            }),
        },
        err => ApiError::Connect(ConnectError::Unreachable {
            host: host_name(),
            reason: err.to_string(),
        }),
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

/// The search for a section's open PRs, newest update first. URL-encoded by
/// hand because each is fixed: `is:pr is:open archived:false` and the
/// section's qualifier, like `review-requested:@me` (directly or through a
/// team).
fn search_path(section: Section) -> &'static str {
    macro_rules! search {
        ($qualifier:literal) => {
            concat!(
                "/search/issues?q=is%3Apr+is%3Aopen+archived%3Afalse+",
                $qualifier,
                "&sort=updated&order=desc&per_page=100"
            )
        };
    }
    match section {
        Section::ReviewRequested => search!("review-requested%3A%40me"),
        Section::Assigned => search!("assignee%3A%40me"),
        Section::Authored => search!("author%3A%40me"),
        Section::Mentioned => search!("mentions%3A%40me"),
    }
}

/// The fields of a search result that an inbox row needs.
#[derive(serde::Deserialize)]
struct SearchItem {
    repository_url: String,
    number: u64,
    title: String,
    user: User,
    created_at: String,
    body: Option<String>,
    /// The PR's GraphQL ID, for fetching its details.
    node_id: Option<String>,
}

impl SearchItem {
    /// The row for this result, or `None` when the last two segments of
    /// `repository_url` aren't a plain `{owner}/{name}`. The response is
    /// untrusted, so the repo goes through the same check as typed input.
    fn into_pull(self, host: &Host) -> Option<PullSummary> {
        let mut segments = self.repository_url.rsplitn(3, '/');
        let (name, owner) = (segments.next()?, segments.next()?);
        let repo = RepoName::parse(&format!("{owner}/{name}")).ok()?;
        Some(PullSummary {
            host: host.as_str().to_string(),
            owner: repo.owner().to_string(),
            repo: repo.name().to_string(),
            number: self.number,
            title: self.title,
            author: self.user.login,
            created_at: self.created_at,
            body: self.body,
            details: None,
        })
    }
}

/// GitHub's search returns at most 1,000 results: 10 pages of 100.
const MAX_PAGES: usize = 10;

/// A host's open PRs in a section, and whether that is all of them.
pub(crate) struct HostPulls {
    pub(crate) pulls: Vec<PullSummary>,
    /// The forge said its results were incomplete, or there were more than
    /// Docent fetched.
    pub(crate) incomplete: bool,
}

/// The path of the next page of a search, relative to `api_base`, or `None`
/// when `next` points anywhere but this host's search. The token goes with
/// every request, so a link elsewhere is never followed.
fn next_search_path(api_base: &str, next: &str) -> Option<String> {
    let rest = next.strip_prefix(api_base.trim_end_matches('/'))?;
    rest.starts_with("/search/issues?")
        .then(|| rest.to_string())
}

/// The open PRs in `section` on `host`, following `next` links up to
/// GitHub's 1,000-result limit, then fetching their details, until
/// `deadline`. If the first page fails the
/// host fails; if a later one does, the pages so far come back as incomplete.
pub(crate) async fn search(
    host: &Host,
    api_base: &str,
    token: &Token,
    section: Section,
    deadline: Instant,
) -> Result<HostPulls, ConnectError> {
    // Each row with its node ID, for the details fetched after the search.
    let mut found: Vec<(Option<String>, PullSummary)> = Vec::new();
    let mut fetched: u64 = 0;
    let mut incomplete = false;
    let mut next = Some(search_path(section).to_string());
    for page_index in 0..MAX_PAGES {
        let Some(path) = next.take() else { break };
        let page: Page<SearchItem> =
            match tokio::time::timeout_at(deadline, get(host, api_base, token, &path)).await {
                Ok(Ok(page)) => page,
                _ if page_index > 0 => {
                    incomplete = true;
                    break;
                }
                Ok(Err(err)) => return Err(err.into_connect(host)),
                Err(_) => {
                    return Err(ConnectError::Unreachable {
                        host: host.as_str().to_string(),
                        reason: "no answer in time".to_string(),
                    });
                }
            };
        fetched += page.items.len() as u64;
        let said_incomplete = page.incomplete_results == Some(true);
        let more_than_linked =
            page.next.is_none() && page.total_count.is_some_and(|total| total > fetched);
        incomplete |= said_incomplete || more_than_linked;
        if let Some(link) = &page.next {
            next = next_search_path(api_base, &link.to_string());
            // A next page Docent won't ask for is a page missing.
            incomplete |= next.is_none();
        }
        found.extend(page.items.into_iter().filter_map(|item| {
            let node_id = item.node_id.clone();
            item.into_pull(host).map(|pull| (node_id, pull))
        }));
    }
    // Still a page to go after the last one Docent fetches.
    incomplete |= next.is_some();
    let ids: Vec<&str> = found.iter().filter_map(|(id, _)| id.as_deref()).collect();
    let mut details = details::fetch(host, api_base, token, &ids, deadline).await;
    let pulls = found
        .into_iter()
        .map(|(id, mut pull)| {
            pull.details = id.and_then(|id| details.remove(&id));
            pull
        })
        .collect();
    Ok(HostPulls { pulls, incomplete })
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

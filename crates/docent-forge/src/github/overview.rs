//! A PR's description, checks and timeline for the Overview tab: one
//! GraphQL query. States are matched as strings, so a value GitHub adds
//! later can't fail the whole response.

use serde::Deserialize;
use serde::de::IgnoredAny;

use super::{ApiError, RepoName, ci_status, graphql_base, post};
use crate::{
    Check, CheckState, ConnectError, Host, PullOverview, ReviewVerdict, TimelineItem, Token,
};

/// The query. It fetches up to 100 checks and the latest 100 timeline items,
/// GitHub's limit per connection: on a long PR, the newest reviews and
/// comments matter most.
const QUERY: &str = "query($owner: String!, $name: String!, $number: Int!) { \
    repository(owner: $owner, name: $name) { pullRequest(number: $number) { \
    title body baseRefOid author { login } \
    commits(last: 1) { nodes { commit { statusCheckRollup { state contexts(first: 100) { nodes { \
        __typename ... on CheckRun { name status conclusion } ... on StatusContext { context state } } } } } } } \
    timelineItems(last: 100, itemTypes: [PULL_REQUEST_COMMIT, PULL_REQUEST_REVIEW, ISSUE_COMMENT]) { \
        pageInfo { hasPreviousPage } nodes { __typename \
        ... on PullRequestCommit { commit { oid messageHeadline committedDate author { name user { login } } } } \
        ... on PullRequestReview { author { login } state body submittedAt } \
        ... on IssueComment { author { login } body createdAt } } } } } }";

/// Shown for an author whose account was deleted, as GitHub does.
const GHOST: &str = "ghost";

#[derive(serde::Serialize)]
struct Request<'a> {
    query: &'static str,
    variables: Variables<'a>,
}

#[derive(serde::Serialize)]
struct Variables<'a> {
    owner: &'a str,
    name: &'a str,
    number: u64,
}

#[derive(Deserialize)]
struct Response {
    /// `null` when the whole query failed.
    data: Option<Data>,
}

#[derive(Deserialize)]
struct Data {
    /// `null` when the repo isn't there or the token can't see it.
    repository: Option<Repository>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    pull_request: Option<PullNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullNode {
    title: String,
    body: String,
    base_ref_oid: String,
    author: Option<Actor>,
    commits: Nodes<CommitNode>,
    timeline_items: Timeline,
}

#[derive(Deserialize)]
struct Actor {
    login: String,
}

/// A login, or `ghost` when the account is gone.
fn login(actor: Option<Actor>) -> String {
    actor.map_or_else(|| GHOST.to_string(), |actor| actor.login)
}

/// `None` for an empty or blank body, which GitHub sends as `""`.
fn non_blank(body: String) -> Option<String> {
    (!body.trim().is_empty()).then_some(body)
}

#[derive(Deserialize)]
struct Nodes<T> {
    nodes: Vec<T>,
}

/// A node Docent can read, or anything else (a type it doesn't show, or one
/// missing a field), which is skipped rather than failing the response.
#[derive(Deserialize)]
#[serde(untagged)]
enum Lenient<T> {
    Known(T),
    Other(IgnoredAny),
}

impl<T> Lenient<T> {
    fn known(self) -> Option<T> {
        match self {
            Self::Known(node) => Some(node),
            Self::Other(_) => None,
        }
    }
}

#[derive(Deserialize)]
struct CommitNode {
    commit: HeadCommit,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeadCommit {
    status_check_rollup: Option<Rollup>,
}

#[derive(Deserialize)]
struct Rollup {
    state: String,
    contexts: Nodes<Lenient<Context>>,
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
enum Context {
    CheckRun {
        name: String,
        status: String,
        conclusion: Option<String>,
    },
    StatusContext {
        context: String,
        state: String,
    },
}

impl Context {
    fn into_check(self) -> Check {
        let (name, state) = match self {
            Self::CheckRun {
                name,
                status,
                conclusion,
            } => {
                let state = match (status.as_str(), conclusion.as_deref()) {
                    ("COMPLETED", Some("SUCCESS")) => CheckState::Passing,
                    (
                        "COMPLETED",
                        Some(
                            "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED"
                            | "STARTUP_FAILURE",
                        ),
                    ) => CheckState::Failing,
                    ("COMPLETED", Some("SKIPPED" | "NEUTRAL" | "STALE")) => CheckState::Skipped,
                    _ => CheckState::Pending,
                };
                (name, state)
            }
            Self::StatusContext { context, state } => {
                let state = match state.as_str() {
                    "SUCCESS" => CheckState::Passing,
                    "FAILURE" | "ERROR" => CheckState::Failing,
                    _ => CheckState::Pending,
                };
                (context, state)
            }
        };
        Check { name, state }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Timeline {
    page_info: PageInfo,
    nodes: Vec<Lenient<Event>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_previous_page: bool,
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
enum Event {
    PullRequestCommit {
        commit: EventCommit,
    },
    #[serde(rename_all = "camelCase")]
    PullRequestReview {
        author: Option<Actor>,
        state: String,
        body: String,
        /// `null` for a pending review.
        submitted_at: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    IssueComment {
        author: Option<Actor>,
        body: String,
        created_at: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventCommit {
    oid: String,
    message_headline: String,
    committed_date: String,
    author: Option<GitActor>,
}

#[derive(Deserialize)]
struct GitActor {
    name: Option<String>,
    user: Option<Actor>,
}

impl Event {
    /// The timeline item, or `None` for a pending review.
    fn into_item(self) -> Option<TimelineItem> {
        match self {
            Self::PullRequestCommit { commit } => {
                let author = commit
                    .author
                    .and_then(|author| author.user.map(|user| user.login).or(author.name));
                Some(TimelineItem::Commit {
                    sha: commit.oid,
                    headline: commit.message_headline,
                    author: author.unwrap_or_else(|| GHOST.to_string()),
                    at: commit.committed_date,
                })
            }
            Self::PullRequestReview {
                author,
                state,
                body,
                submitted_at,
            } => {
                let verdict = match state.as_str() {
                    "APPROVED" => ReviewVerdict::Approved,
                    "CHANGES_REQUESTED" => ReviewVerdict::ChangesRequested,
                    "COMMENTED" => ReviewVerdict::Commented,
                    "DISMISSED" => ReviewVerdict::Dismissed,
                    // `PENDING`: the viewer's own draft, not yet submitted.
                    _ => return None,
                };
                Some(TimelineItem::Review {
                    author: login(author),
                    verdict,
                    body: non_blank(body),
                    at: submitted_at?,
                })
            }
            Self::IssueComment {
                author,
                body,
                created_at,
            } => Some(TimelineItem::Comment {
                author: login(author),
                body,
                at: created_at,
            }),
        }
    }
}

impl PullNode {
    fn into_overview(self) -> PullOverview {
        let rollup = self
            .commits
            .nodes
            .into_iter()
            .last()
            .and_then(|node| node.commit.status_check_rollup);
        let ci = ci_status(rollup.as_ref().map(|rollup| rollup.state.as_str()));
        let checks = rollup
            .map(|rollup| rollup.contexts.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter_map(Lenient::known)
            .map(Context::into_check)
            .collect();
        PullOverview {
            title: self.title,
            author: login(self.author),
            body: non_blank(self.body),
            base_sha: self.base_ref_oid,
            ci,
            checks,
            timeline: self
                .timeline_items
                .nodes
                .into_iter()
                .filter_map(Lenient::known)
                .filter_map(Event::into_item)
                .collect(),
            timeline_truncated: self.timeline_items.page_info.has_previous_page,
        }
    }
}

/// The overview of PR `number` in `repo`. A repo or PR GitHub doesn't show
/// this token is [`ApiError::NotFound`]; a query that failed as a whole is
/// unreachable, without GitHub's message, which the server controls.
pub(crate) async fn pull_overview(
    host: &Host,
    api_base: &str,
    token: &Token,
    repo: &RepoName,
    number: u64,
) -> Result<PullOverview, ApiError> {
    let request = Request {
        query: QUERY,
        variables: Variables {
            owner: repo.owner(),
            name: repo.name(),
            number,
        },
    };
    let response: Response =
        post(host, graphql_base(api_base), token, "/graphql", &request).await?;
    let Some(data) = response.data else {
        return Err(ApiError::Connect(ConnectError::Unreachable {
            host: host.as_str().to_string(),
            reason: "GitHub could not answer the query".to_string(),
        }));
    };
    data.repository
        .and_then(|repo| repo.pull_request)
        .map(PullNode::into_overview)
        .ok_or(ApiError::NotFound)
}

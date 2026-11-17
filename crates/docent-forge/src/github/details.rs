//! CI status, review state and size for inbox rows: one GraphQL query per
//! 100 PRs, by the node IDs the search returned.

use std::collections::HashMap;

use serde::de::IgnoredAny;
use tokio::time::Instant;

use super::{ci_status, client, graphql_base};
use crate::{Host, PullDetails, ReviewState, Token};

/// GraphQL's `nodes` takes at most 100 IDs.
const MAX_NODE_IDS: usize = 100;

const QUERY: &str = "query($ids: [ID!]!) { nodes(ids: $ids) { ... on PullRequest { \
    id additions deletions changedFiles reviewDecision reviews(first: 0, states: [APPROVED, CHANGES_REQUESTED, COMMENTED, DISMISSED]) { totalCount } \
    commits(last: 1) { nodes { commit { statusCheckRollup { state } } } } } } }";

#[derive(serde::Serialize)]
struct Request<'a> {
    query: &'static str,
    variables: Variables<'a>,
}

#[derive(serde::Serialize)]
struct Variables<'a> {
    ids: &'a [&'a str],
}

#[derive(serde::Deserialize)]
struct Response {
    /// `null` when the whole query failed.
    data: Option<Data>,
}

#[derive(serde::Deserialize)]
struct Data {
    /// `null` for an ID GitHub couldn't resolve.
    nodes: Vec<Option<Node>>,
}

/// A node that is a PR, or anything else, which is skipped.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Node {
    Pull(Box<PullNode>),
    Other(IgnoredAny),
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullNode {
    id: String,
    additions: u64,
    deletions: u64,
    changed_files: u64,
    review_decision: Option<String>,
    /// Submitted reviews only: the viewer's pending draft doesn't count.
    reviews: Count,
    commits: Nodes<CommitNode>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Count {
    total_count: u64,
}

#[derive(serde::Deserialize)]
struct Nodes<T> {
    nodes: Vec<T>,
}

#[derive(serde::Deserialize)]
struct CommitNode {
    commit: Commit,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    status_check_rollup: Option<Rollup>,
}

#[derive(serde::Deserialize)]
struct Rollup {
    state: String,
}

impl PullNode {
    fn details(&self) -> PullDetails {
        let rollup = self
            .commits
            .nodes
            .last()
            .and_then(|node| node.commit.status_check_rollup.as_ref());
        let ci = ci_status(rollup.map(|rollup| rollup.state.as_str()));
        let review = match self.review_decision.as_deref() {
            Some("APPROVED") => ReviewState::Approved,
            Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
            Some("REVIEW_REQUIRED") => ReviewState::ReviewRequired,
            // No decision: the repo doesn't require reviews.
            _ if self.reviews.total_count > 0 => ReviewState::Reviewed,
            _ => ReviewState::NoReviews,
        };
        PullDetails {
            ci,
            review,
            additions: self.additions,
            deletions: self.deletions,
            changed_files: self.changed_files,
        }
    }
}

/// The details of the PRs with node IDs `ids`, keyed by ID, until
/// `deadline`. They're extras, so failures aren't reported: the first batch
/// that fails ends the fetch, since later ones would most likely fail the
/// same way, and the rows left show without details.
pub(crate) async fn fetch(
    host: &Host,
    api_base: &str,
    token: &Token,
    ids: &[&str],
    deadline: Instant,
) -> HashMap<String, PullDetails> {
    let mut found = HashMap::new();
    let Ok(client) = client(host, graphql_base(api_base), token) else {
        return found;
    };
    for batch in ids.chunks(MAX_NODE_IDS) {
        let request = Request {
            query: QUERY,
            variables: Variables { ids: batch },
        };
        let answer = tokio::time::timeout_at(
            deadline,
            client.post::<_, Response>("/graphql", Some(&request)),
        )
        .await;
        let Ok(Ok(Response { data: Some(data) })) = answer else {
            break;
        };
        // Rows look their details up by their own ID, so a node Docent
        // didn't ask for is never shown.
        for node in data.nodes.into_iter().flatten() {
            if let Node::Pull(pull) = node {
                found.insert(pull.id.clone(), pull.details());
            }
        }
    }
    found
}

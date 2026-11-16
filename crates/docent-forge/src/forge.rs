//! The `Forge` trait: what Docent asks of a forge, across every connected
//! host. GitHub (through [`crate::Accounts`]) is the only implementation for
//! now; the test fake keeps it from being shaped around GitHub alone.

use std::future::Future;

use crate::ConnectError;

/// An open pull request, as an inbox row shows it. Everything here came from
/// the forge, so the UI shows it as text only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PullSummary {
    pub host: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    /// The author's login.
    pub author: String,
    /// When the PR was opened, as the forge's RFC 3339 timestamp.
    pub created_at: String,
    /// The PR's description as the author wrote it (Markdown), or `None`
    /// when it has none. Untrusted: shown as text, never as HTML.
    pub body: Option<String>,
    /// CI status, review state and size, or `None` when the forge couldn't
    /// say. They're extras: a row without them still shows.
    pub details: Option<PullDetails>,
}

/// What an inbox row shows beyond the search result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PullDetails {
    pub ci: CiStatus,
    pub review: ReviewState,
    /// Lines added and deleted, and files changed, across the whole PR.
    pub additions: u64,
    pub deletions: u64,
    pub changed_files: u64,
}

/// The combined state of the checks on a PR's head commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStatus {
    Passing,
    Failing,
    /// Still running, or waiting on a check that hasn't reported.
    Pending,
    /// The head commit has no checks or statuses.
    NoChecks,
}

/// Where a PR stands with its reviewers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    /// The repo requires an approving review and there isn't one yet.
    ReviewRequired,
    /// Reviewed, in a repo that doesn't require reviews, so there's no
    /// decision.
    Reviewed,
    /// No reviews, in a repo that doesn't require them.
    NoReviews,
}

/// A host that could not be asked, and why.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HostFailure {
    pub host: String,
    pub error: ConnectError,
}

/// An inbox section the forge searches for. The UI names one in snake case,
/// like `review_requested`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    /// Open PRs that request the user's review, directly or through a team.
    ReviewRequested,
    /// Open PRs assigned to the user.
    Assigned,
    /// Open PRs the user opened.
    Authored,
    /// Open PRs that mention the user.
    Mentioned,
}

/// The PRs found on the hosts that answered, and the hosts that didn't.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct SectionPulls {
    pub pulls: Vec<PullSummary>,
    pub failures: Vec<HostFailure>,
    /// Hosts that answered, but whose results were cut short: the forge said
    /// so, or there were more than Docent fetches.
    pub incomplete: Vec<String>,
}

/// A forge, across every connected host.
pub trait Forge: Sync {
    /// The open PRs in `section`, from every connected host. Each host is
    /// asked on its own, so one failing host doesn't hide the others.
    fn search(&self, section: Section) -> impl Future<Output = SectionPulls> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ui_receives_pulls_failures_and_incomplete_hosts() {
        let found = SectionPulls {
            pulls: vec![PullSummary {
                host: "github.com".to_string(),
                owner: "octocat".to_string(),
                repo: "Hello-World".to_string(),
                number: 1347,
                title: "Fix the greeting".to_string(),
                author: "mona".to_string(),
                created_at: "2026-11-10T09:00:00Z".to_string(),
                body: Some("Say hello properly.".to_string()),
                details: None,
            }],
            failures: vec![HostFailure {
                host: "ghe.corp.test".to_string(),
                error: ConnectError::HttpStatus {
                    host: "ghe.corp.test".to_string(),
                    status: 503,
                },
            }],
            incomplete: vec!["github.com".to_string()],
        };

        assert_eq!(
            serde_json::to_value(&found).unwrap(),
            serde_json::json!({
                "pulls": [{
                    "host": "github.com",
                    "owner": "octocat",
                    "repo": "Hello-World",
                    "number": 1347,
                    "title": "Fix the greeting",
                    "author": "mona",
                    "created_at": "2026-11-10T09:00:00Z",
                    "body": "Say hello properly.",
                    "details": null,
                }],
                "failures": [{
                    "host": "ghe.corp.test",
                    "error": {
                        "kind": "http_status",
                        "message": "ghe.corp.test answered with HTTP 503.",
                    },
                }],
                "incomplete": ["github.com"],
            })
        );
    }
}

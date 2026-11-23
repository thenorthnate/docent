//! What the review page's Overview tab shows for one PR. Every string here
//! came from the forge, so the UI treats it as untrusted.

use crate::CiStatus;

/// A PR's description, checks and timeline.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PullOverview {
    pub title: String,
    /// The author's login, or `ghost` for a deleted account.
    pub author: String,
    /// The description as Markdown, or `None` when it is empty.
    pub body: Option<String>,
    /// The combined state of the head commit's checks.
    pub ci: CiStatus,
    /// The head commit's checks, in the forge's order.
    pub checks: Vec<Check>,
    /// The latest commits, reviews and comments, oldest first.
    pub timeline: Vec<TimelineItem>,
    /// There are earlier timeline items than Docent fetched.
    pub timeline_truncated: bool,
}

/// One check on the head commit: a check run or a commit status.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Check {
    pub name: String,
    pub state: CheckState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Passing,
    Failing,
    /// Queued, running, or a state Docent doesn't know.
    Pending,
    /// Finished without a verdict: skipped, neutral or stale.
    Skipped,
}

/// An event on the PR's timeline. Times are the forge's RFC 3339 timestamps.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineItem {
    Commit {
        /// The full commit hash.
        sha: String,
        /// The first line of the commit message.
        headline: String,
        /// The author's login, or their git name when no account matches.
        author: String,
        at: String,
    },
    Review {
        author: String,
        verdict: ReviewVerdict,
        /// `None` when the review has no summary.
        body: Option<String>,
        at: String,
    },
    Comment {
        author: String,
        body: String,
        at: String,
    },
}

/// What a submitted review said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Approved,
    ChangesRequested,
    Commented,
    Dismissed,
}

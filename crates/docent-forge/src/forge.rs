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
}

/// A host that could not be asked, and why.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HostFailure {
    pub host: String,
    pub error: ConnectError,
}

/// The PRs found on the hosts that answered, and the hosts that didn't.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct ReviewRequested {
    pub pulls: Vec<PullSummary>,
    pub failures: Vec<HostFailure>,
}

/// A forge, across every connected host.
pub trait Forge: Sync {
    /// Open PRs that request the user's review, from every connected host.
    /// Each host is asked on its own, so one failing host doesn't hide the
    /// others.
    fn review_requested(&self) -> impl Future<Output = ReviewRequested> + Send;
}

//! A fake [`Forge`] with canned answers, for tests in other crates. Built only
//! with the `fake` feature, which nothing turns on outside dev-dependencies.

use std::sync::{Mutex, PoisonError};

use crate::github::RepoName;
use crate::{Forge, PullOverview, PullRef, PullStates, RepoError, Section, SectionPulls};

/// Answers [`Forge::search`] with the same fixed value for every section,
/// [`Forge::pull_overview`] with a fixed overview for any PR, and
/// [`Forge::pull_states`] with the fixed states of the PRs asked about, and
/// records what was asked.
#[derive(Debug, Default)]
pub struct FakeForge {
    pulls: SectionPulls,
    overview: Option<PullOverview>,
    asked: Mutex<Vec<Section>>,
    overviews_asked: Mutex<Vec<String>>,
    states: PullStates,
    states_asked: Mutex<Vec<Vec<PullRef>>>,
}

impl FakeForge {
    pub fn with_pulls(pulls: SectionPulls) -> Self {
        Self {
            pulls,
            ..Self::default()
        }
    }

    /// A forge that answers every overview with `overview`. Without one,
    /// every PR is not found.
    pub fn with_overview(overview: PullOverview) -> Self {
        Self {
            overview: Some(overview),
            ..Self::default()
        }
    }

    /// A forge that knows the states in `states`, and fails the hosts in its
    /// `failures`, whose PRs it reports no states for.
    pub fn with_states(states: PullStates) -> Self {
        Self {
            states,
            ..Self::default()
        }
    }

    /// The PRs each call to [`Forge::pull_states`] asked about, in order.
    pub fn states_asked(&self) -> Vec<Vec<PullRef>> {
        self.states_asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The sections searched so far, in order.
    pub fn asked(&self) -> Vec<Section> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The overviews asked for so far, as `host/owner/name#number`.
    pub fn overviews_asked(&self) -> Vec<String> {
        self.overviews_asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Forge for FakeForge {
    async fn search(&self, section: Section) -> SectionPulls {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(section);
        self.pulls.clone()
    }

    async fn pull_overview(
        &self,
        host: &str,
        repo: &RepoName,
        number: u64,
    ) -> Result<PullOverview, RepoError> {
        let name = repo.to_string();
        self.overviews_asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("{host}/{name}#{number}"));
        self.overview.clone().ok_or(RepoError::PullNotFound {
            host: host.to_string(),
            repo: name,
            number,
        })
    }

    async fn pull_states(&self, pulls: &[PullRef]) -> PullStates {
        self.states_asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(pulls.to_vec());
        let failed = |host: &str| self.states.failures.iter().any(|f| f.host == host);
        PullStates {
            states: self
                .states
                .states
                .iter()
                .filter(|(pull, _)| pulls.contains(pull) && !failed(&pull.host))
                .cloned()
                .collect(),
            failures: self.states.failures.clone(),
        }
    }
}

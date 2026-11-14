//! A fake [`Forge`] with canned answers, for tests in other crates. Built only
//! with the `fake` feature, which nothing turns on outside dev-dependencies.

use std::sync::{Mutex, PoisonError};

use crate::{Forge, Section, SectionPulls};

/// Answers [`Forge::search`] with the same fixed value for every section, and
/// records which sections were asked for.
#[derive(Debug, Default)]
pub struct FakeForge {
    pulls: SectionPulls,
    asked: Mutex<Vec<Section>>,
}

impl FakeForge {
    pub fn with_pulls(pulls: SectionPulls) -> Self {
        Self {
            pulls,
            asked: Mutex::default(),
        }
    }

    /// The sections searched so far, in order.
    pub fn asked(&self) -> Vec<Section> {
        self.asked
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
}

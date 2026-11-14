//! A fake [`Forge`] with canned answers, for tests in other crates. Built only
//! with the `fake` feature, which nothing turns on outside dev-dependencies.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{Forge, ReviewRequested};

/// Answers [`Forge::review_requested`] with a fixed value and counts the calls.
#[derive(Debug, Default)]
pub struct FakeForge {
    review_requested: ReviewRequested,
    calls: AtomicUsize,
}

impl FakeForge {
    pub fn with_review_requested(review_requested: ReviewRequested) -> Self {
        Self {
            review_requested,
            calls: AtomicUsize::new(0),
        }
    }

    /// How many times the forge has been asked anything.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Forge for FakeForge {
    async fn review_requested(&self) -> ReviewRequested {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.review_requested.clone()
    }
}

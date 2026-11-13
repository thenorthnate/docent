//! Docent's local state in SQLite: connected accounts and watched repos.

use std::path::Path;

/// The row id of an account.
pub type AccountId = i64;
/// The row id of a repo.
pub type RepoId = i64;

/// A repo to watch, as the forge reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRepo {
    pub owner: String,
    pub name: String,
    pub default_branch: String,
}

/// A watched repo, as the UI sees it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WatchedRepo {
    pub id: RepoId,
    pub host: String,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    /// The bound local clone, if any.
    pub local_path: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("{0} is already watched.")]
    AlreadyWatched(String),
    #[error("No watched repo has id {0}.")]
    RepoNotFound(RepoId),
    #[error("Docent's database failed: {0}")]
    Database(String),
}

pub struct Store {}

impl Store {
    pub async fn open(_path: &Path) -> Result<Self, StoreError> {
        todo!()
    }

    pub async fn open_in_memory() -> Result<Self, StoreError> {
        todo!()
    }

    pub async fn close(self) {
        todo!()
    }

    pub async fn upsert_account(&self, _host: &str, _login: &str) -> Result<AccountId, StoreError> {
        todo!()
    }

    pub async fn watch_repo(
        &self,
        _account: AccountId,
        _repo: &NewRepo,
    ) -> Result<WatchedRepo, StoreError> {
        todo!()
    }

    pub async fn watched_repos(&self) -> Result<Vec<WatchedRepo>, StoreError> {
        todo!()
    }

    pub async fn unwatch_repo(&self, _id: RepoId) -> Result<(), StoreError> {
        todo!()
    }
}

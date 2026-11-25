//! Docent's local state in SQLite: connected accounts and watched repos.
//!
//! Queries are plain SQL checked at runtime, so building needs no database.
//! The schema lives in `migrations/`, which run every time a store opens.

use std::path::Path;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, sqlx::FromRow)]
pub struct WatchedRepo {
    pub id: RepoId,
    pub host: String,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    /// The bound local clone, if any.
    pub local_path: Option<String>,
}

/// A connected host as remembered across launches: its name and login only.
/// There is no token here, nor anywhere in the store; tokens are read from
/// `gh` again at startup.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SavedAccount {
    pub host: String,
    pub login: String,
}

/// A PR worktree Docent created.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RecordedWorktree {
    pub repo_id: RepoId,
    #[sqlx(try_from = "i64")]
    pub number: u64,
    /// The clone it came from, whatever the repo is bound to now.
    pub clone_path: String,
    /// The remote of that clone its PR was fetched from.
    pub remote: String,
}

/// How many bytes PR worktrees may use until the user chooses.
pub const DEFAULT_WORKTREE_CAP: u64 = 10_000_000_000;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("{0} is already watched.")]
    AlreadyWatched(String),
    #[error("No watched repo has id {0}.")]
    RepoNotFound(RepoId),
    #[error("{0} isn't watched. Watch it under Inbox → Watched repos to review its PRs.")]
    NotWatched(String),
    #[error("Docent's database failed: {0}")]
    Database(String),
}

impl StoreError {
    /// A stable name for the UI to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::AlreadyWatched(_) => "already_watched",
            Self::RepoNotFound(_) => "repo_not_found",
            Self::NotWatched(_) => "not_watched",
            Self::Database(_) => "database",
        }
    }
}

impl From<sqlx::Error> for StoreError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for StoreError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Self::Database(err.to_string())
    }
}

/// Selects the columns of a [`WatchedRepo`], joined with its account's host.
/// A macro, so `concat!` can build each query as a `&'static str`: sqlx only
/// runs SQL fixed at compile time, which rules out injection.
macro_rules! select_repo {
    ($($rest:literal),+) => {
        concat!(
            "SELECT repos.id, accounts.host, repos.owner, repos.name, ",
            "repos.default_branch, repos.local_path ",
            "FROM repos JOIN accounts ON accounts.id = repos.account_id ",
            $($rest),+
        )
    };
}

#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Opens the database at `path`, creating it and its folder if needed,
    /// and brings its schema up to date.
    pub async fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|err| StoreError::Database(format!("{}: {err}", dir.display())))?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);
        Self::migrated(SqlitePoolOptions::new().connect_with(options).await?).await
    }

    /// A fresh database that lives as long as the store. For tests.
    pub async fn open_in_memory() -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true);
        // Each connection to `:memory:` is its own database, so keep exactly
        // one, open for good.
        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        Self::migrated(pool).await
    }

    async fn migrated(pool: SqlitePool) -> Result<Self, StoreError> {
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Closes every connection, waiting for queries in flight.
    pub async fn close(self) {
        self.pool.close().await;
    }

    /// Records the GitHub login connected on `host` through `gh`, replacing
    /// the login if the host is already recorded. Returns the account's id.
    pub async fn upsert_account(&self, host: &str, login: &str) -> Result<AccountId, StoreError> {
        let id = sqlx::query_scalar(
            "INSERT INTO accounts (forge, host, login, auth_source) \
             VALUES ('github', ?1, ?2, 'gh') \
             ON CONFLICT (forge, host) DO UPDATE SET login = excluded.login \
             RETURNING id",
        )
        .bind(host)
        .bind(login)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// The remembered hosts, sorted by host name.
    pub async fn saved_accounts(&self) -> Result<Vec<SavedAccount>, StoreError> {
        let accounts = sqlx::query_as("SELECT host, login FROM accounts ORDER BY host")
            .fetch_all(&self.pool)
            .await?;
        Ok(accounts)
    }

    /// Watches `repo` on `account`. A repo that was unwatched is watched
    /// again, keeping its row; one already watched is rejected.
    pub async fn watch_repo(
        &self,
        account: AccountId,
        repo: &NewRepo,
    ) -> Result<WatchedRepo, StoreError> {
        // The update only applies to an unwatched row, so a watched one
        // returns nothing and is reported as a duplicate.
        let id: Option<RepoId> = sqlx::query_scalar(
            "INSERT INTO repos (account_id, owner, name, default_branch) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (account_id, owner, name) DO UPDATE SET \
                 watched = 1, owner = excluded.owner, name = excluded.name, \
                 default_branch = excluded.default_branch \
             WHERE repos.watched = 0 \
             RETURNING id",
        )
        .bind(account)
        .bind(&repo.owner)
        .bind(&repo.name)
        .bind(&repo.default_branch)
        .fetch_optional(&self.pool)
        .await?;

        match id {
            Some(id) => self.repo(id).await,
            None => {
                let existing: WatchedRepo = sqlx::query_as(select_repo!(
                    "WHERE repos.account_id = ?1 AND repos.owner = ?2 AND repos.name = ?3"
                ))
                .bind(account)
                .bind(&repo.owner)
                .bind(&repo.name)
                .fetch_one(&self.pool)
                .await?;
                Err(StoreError::AlreadyWatched(format!(
                    "{}:{}/{}",
                    existing.host, existing.owner, existing.name
                )))
            }
        }
    }

    /// The watched repos, sorted by host, owner and name.
    pub async fn watched_repos(&self) -> Result<Vec<WatchedRepo>, StoreError> {
        let repos = sqlx::query_as(select_repo!(
            "WHERE repos.watched = 1 ",
            "ORDER BY accounts.host, repos.owner, repos.name"
        ))
        .fetch_all(&self.pool)
        .await?;
        Ok(repos)
    }

    /// Stops watching a repo. Its row, and any binding, stay. Unwatching a
    /// repo that is already unwatched does nothing.
    pub async fn unwatch_repo(&self, id: RepoId) -> Result<(), StoreError> {
        let result = sqlx::query("UPDATE repos SET watched = 0 WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::RepoNotFound(id));
        }
        Ok(())
    }

    /// The watched repo with this id.
    pub async fn watched_repo(&self, id: RepoId) -> Result<WatchedRepo, StoreError> {
        sqlx::query_as(select_repo!("WHERE repos.id = ?1 AND repos.watched = 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::RepoNotFound(id))
    }

    /// The watched repo `owner/name` on `host`. Names match in any case, as
    /// on the forge.
    pub async fn watched_repo_named(
        &self,
        host: &str,
        owner: &str,
        name: &str,
    ) -> Result<WatchedRepo, StoreError> {
        sqlx::query_as(select_repo!(
            "WHERE accounts.host = ?1 AND repos.owner = ?2 AND repos.name = ?3 ",
            "AND repos.watched = 1"
        ))
        .bind(host)
        .bind(owner)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotWatched(format!("{host}:{owner}/{name}")))
    }

    /// Binds a watched repo to the local clone at `local_path`, replacing
    /// any earlier binding. The caller checks the clone first.
    pub async fn bind_repo(&self, id: RepoId, local_path: &str) -> Result<WatchedRepo, StoreError> {
        let result = sqlx::query("UPDATE repos SET local_path = ?2 WHERE id = ?1 AND watched = 1")
            .bind(id)
            .bind(local_path)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::RepoNotFound(id));
        }
        self.watched_repo(id).await
    }

    /// Records that PR `number`'s worktree is being opened now, from `remote`
    /// of the clone at `clone_path`.
    pub async fn record_worktree(
        &self,
        repo: RepoId,
        number: u64,
        clone_path: &str,
        remote: &str,
    ) -> Result<(), StoreError> {
        // Always later than every other row, even within a millisecond or
        // after the clock goes back, so the order is the order of use.
        sqlx::query(
            "INSERT INTO worktrees (repo_id, number, clone_path, remote, last_opened) \
             VALUES (?1, ?2, ?3, ?4, MAX( \
                 CAST(unixepoch('subsec') * 1000 AS INTEGER), \
                 (SELECT COALESCE(MAX(last_opened), 0) + 1 FROM worktrees))) \
             ON CONFLICT (repo_id, number) DO UPDATE SET \
                 clone_path = excluded.clone_path, remote = excluded.remote, \
                 last_opened = excluded.last_opened",
        )
        .bind(repo)
        .bind(pr_number(number)?)
        .bind(clone_path)
        .bind(remote)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The recorded worktrees, least recently opened first.
    pub async fn worktrees(&self) -> Result<Vec<RecordedWorktree>, StoreError> {
        let worktrees = sqlx::query_as(
            "SELECT repo_id, number, clone_path, remote FROM worktrees ORDER BY last_opened",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(worktrees)
    }

    /// Drops a worktree's record, once it is removed.
    pub async fn forget_worktree(&self, repo: RepoId, number: u64) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM worktrees WHERE repo_id = ?1 AND number = ?2")
            .bind(repo)
            .bind(pr_number(number)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// How many bytes PR worktrees may use.
    pub async fn worktree_cap(&self) -> Result<u64, StoreError> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'worktree_cap_bytes'")
                .fetch_optional(&self.pool)
                .await?;
        match value {
            None => Ok(DEFAULT_WORKTREE_CAP),
            Some(value) => value.parse().map_err(|_| {
                StoreError::Database(format!("the worktree disk cap isn't a number: {value}"))
            }),
        }
    }

    pub async fn set_worktree_cap(&self, bytes: u64) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES ('worktree_cap_bytes', ?1) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        )
        .bind(bytes.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn repo(&self, id: RepoId) -> Result<WatchedRepo, StoreError> {
        let repo = sqlx::query_as(select_repo!("WHERE repos.id = ?1"))
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(repo)
    }
}

/// SQLite integers are signed. No forge numbers PRs past `i64`.
fn pr_number(number: u64) -> Result<i64, StoreError> {
    i64::try_from(number)
        .map_err(|_| StoreError::Database(format!("PR number {number} is too large")))
}

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use crate::github::{self, RepoName};
use crate::{ConnectError, ForgeRepo, GhCli, Host, RepoError, Token};

type ApiBase = Box<dyn Fn(&Host) -> String + Send + Sync>;

/// A connected host as the UI sees it. It never holds the token.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AccountSummary {
    pub host: String,
    pub login: String,
    /// Why a remembered host failed to reconnect, or `None` when connected.
    pub error: Option<ConnectError>,
}

#[derive(Debug)]
struct Account {
    login: String,
    /// Shared so an API call can use it without holding the lock.
    token: Arc<Token>,
}

impl Account {
    fn summary(&self, host: &Host) -> AccountSummary {
        AccountSummary {
            host: host.as_str().to_string(),
            login: self.login.clone(),
            error: None,
        }
    }
}

/// The connected forge hosts. Held in memory only, so nothing is written to
/// disk and the user reconnects after a restart (remembering hosts across
/// launches is a later slice).
pub struct Accounts {
    api_base: ApiBase,
    by_host: Mutex<BTreeMap<Host, Account>>,
}

impl Default for Accounts {
    fn default() -> Self {
        Self::with_api_base(Host::api_base)
    }
}

impl std::fmt::Debug for Accounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Accounts")
            .field("by_host", &self.by_host)
            .finish_non_exhaustive()
    }
}

impl Accounts {
    /// Accounts whose API lives at `api_base(host)`. Tests point it at a mock.
    pub fn with_api_base(api_base: impl Fn(&Host) -> String + Send + Sync + 'static) -> Self {
        Self {
            api_base: Box::new(api_base),
            by_host: Mutex::default(),
        }
    }

    /// Connects `host` with the token from `gh`, replacing any earlier
    /// connection to it.
    pub async fn connect(&self, host: &str, gh: &GhCli) -> Result<AccountSummary, ConnectError> {
        let host = Host::parse(host)?;
        let token = gh.token(&host).await?;
        let login = github::current_login(&host, &(self.api_base)(&host), &token).await?;
        let account = Account {
            login,
            token: Arc::new(token),
        };
        let summary = account.summary(&host);
        self.lock().insert(host, account);
        Ok(summary)
    }

    /// Reconnects a host remembered from an earlier launch, with a fresh
    /// token from `gh`. If that fails, the host stays listed under `login`
    /// with the error, so the user sees it and can reconnect.
    pub async fn restore(
        &self,
        _host: &str,
        _login: &str,
        _gh: &GhCli,
    ) -> Result<AccountSummary, ConnectError> {
        todo!()
    }

    /// Looks `repo` up on `host` with that host's token, to confirm it exists
    /// and get its canonical name.
    pub async fn lookup_repo(&self, host: &str, repo: &RepoName) -> Result<ForgeRepo, RepoError> {
        let host = Host::parse(host)?;
        let (account, token) = {
            let by_host = self.lock();
            let account = by_host
                .get(&host)
                .ok_or_else(|| RepoError::NotConnected(host.as_str().to_string()))?;
            (account.summary(&host), Arc::clone(&account.token))
        };
        match github::repo(&host, &(self.api_base)(&host), &token, repo).await {
            Ok(found) => Ok(ForgeRepo {
                account,
                owner: found.owner_login().to_string(),
                name: found.name,
                default_branch: found.default_branch,
            }),
            Err(github::ApiError::NotFound) => Err(RepoError::NotFound {
                host: host.as_str().to_string(),
                repo: format!("{}/{}", repo.owner(), repo.name()),
            }),
            Err(github::ApiError::Connect(err)) => Err(err.into()),
        }
    }

    /// The connected hosts, sorted by host name.
    pub fn list(&self) -> Vec<AccountSummary> {
        self.lock()
            .iter()
            .map(|(host, account)| account.summary(host))
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<Host, Account>> {
        // The map is only ever replaced whole-entry, so it stays consistent
        // even if a holder panicked.
        self.by_host.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

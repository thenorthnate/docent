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

/// A host the user connected: live with a token, or remembered from an
/// earlier launch but not reconnected.
#[derive(Debug)]
enum Connection {
    Connected(Account),
    Failed { login: String, error: ConnectError },
}

impl Connection {
    fn summary(&self, host: &Host) -> AccountSummary {
        match self {
            Self::Connected(account) => account.summary(host),
            Self::Failed { login, error } => AccountSummary {
                host: host.as_str().to_string(),
                login: login.clone(),
                error: Some(error.clone()),
            },
        }
    }
}

/// The user's forge hosts. Tokens are held in memory only. Which hosts are
/// connected is remembered by the caller (`docent-store`), never here.
pub struct Accounts {
    api_base: ApiBase,
    by_host: Mutex<BTreeMap<Host, Connection>>,
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
        let connection = Connection::Connected(Account {
            login,
            token: Arc::new(token),
        });
        let summary = connection.summary(&host);
        self.lock().insert(host, connection);
        Ok(summary)
    }

    /// Reconnects a host remembered from an earlier launch, with a fresh
    /// token from `gh`. If that fails, the host stays listed under `login`
    /// with the error, so the user sees it and can reconnect.
    ///
    /// `host` comes from disk, so it is parsed like typed input: a host that
    /// isn't a plain host name never reaches `gh` and isn't listed.
    pub async fn restore(
        &self,
        host: &str,
        login: &str,
        gh: &GhCli,
    ) -> Result<AccountSummary, ConnectError> {
        let host = Host::parse(host)?;
        let err = match self.connect(host.as_str(), gh).await {
            Ok(summary) => return Ok(summary),
            Err(err) => err,
        };
        let mut by_host = self.lock();
        // A live connection made meanwhile wins over the failed restore.
        if !matches!(by_host.get(&host), Some(Connection::Connected(_))) {
            by_host.insert(
                host,
                Connection::Failed {
                    login: login.to_string(),
                    error: err.clone(),
                },
            );
        }
        Err(err)
    }

    /// Looks `repo` up on `host` with that host's token, to confirm it exists
    /// and get its canonical name.
    pub async fn lookup_repo(&self, host: &str, repo: &RepoName) -> Result<ForgeRepo, RepoError> {
        let host = Host::parse(host)?;
        let (account, token) = {
            let by_host = self.lock();
            let Some(Connection::Connected(account)) = by_host.get(&host) else {
                return Err(RepoError::NotConnected(host.as_str().to_string()));
            };
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

    /// The user's hosts, sorted by host name. A host that failed to
    /// reconnect carries its error.
    pub fn list(&self) -> Vec<AccountSummary> {
        self.lock()
            .iter()
            .map(|(host, connection)| connection.summary(host))
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<Host, Connection>> {
        // The map is only ever replaced whole-entry, so it stays consistent
        // even if a holder panicked.
        self.by_host.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

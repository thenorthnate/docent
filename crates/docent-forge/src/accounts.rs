use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use crate::{ConnectError, GhCli, Host, Token, github};

type ApiBase = Box<dyn Fn(&Host) -> String + Send + Sync>;

/// A connected host as the UI sees it. It never holds the token.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AccountSummary {
    pub host: String,
    pub login: String,
}

#[derive(Debug)]
struct Account {
    login: String,
    // Read by the forge API calls that later slices add.
    #[allow(dead_code)]
    token: Token,
}

/// The connected forge hosts. Held in memory only: tokens are fetched from
/// `gh` again on every launch and never written to disk.
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
        let summary = AccountSummary {
            host: host.as_str().to_string(),
            login: login.clone(),
        };
        self.lock().insert(host, Account { login, token });
        Ok(summary)
    }

    /// The connected hosts, sorted by host name.
    pub fn list(&self) -> Vec<AccountSummary> {
        self.lock()
            .iter()
            .map(|(host, account)| AccountSummary {
                host: host.as_str().to_string(),
                login: account.login.clone(),
            })
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<Host, Account>> {
        // The map is only ever replaced whole-entry, so it stays consistent
        // even if a holder panicked.
        self.by_host.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

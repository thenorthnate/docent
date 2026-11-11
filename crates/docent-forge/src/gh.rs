use std::ffi::OsString;
use std::io::ErrorKind;
use std::process::Stdio;

use tokio::process::Command;

use crate::{ConnectError, Host, Token};

/// The user's GitHub CLI. This module is the only place Docent runs `gh`.
#[derive(Debug, Clone)]
pub struct GhCli {
    path: OsString,
}

impl GhCli {
    /// `path` is the `PATH` to find `gh` on, normally the login shell's,
    /// since an app launched from Finder doesn't inherit it.
    pub fn new(path: OsString) -> Self {
        Self { path }
    }

    /// Runs `gh auth token --hostname <host>`.
    pub(crate) async fn token(&self, host: &Host) -> Result<Token, ConnectError> {
        let output = Command::new("gh")
            .args(["auth", "token", "--hostname", host.as_str()])
            .env("PATH", &self.path)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|err| match err.kind() {
                ErrorKind::NotFound => ConnectError::GhNotFound,
                _ => ConnectError::GhFailed(err.to_string()),
            })?;

        // gh exits non-zero when it has no token for the host. Its stderr is
        // not passed on: the hint in `NotLoggedIn` is the actionable part.
        let token = String::from_utf8(output.stdout)
            .ok()
            .map(|out| out.trim().to_string())
            .filter(|token| output.status.success() && !token.is_empty());
        token
            .map(Token::new)
            .ok_or_else(|| ConnectError::NotLoggedIn(host.as_str().to_string()))
    }
}

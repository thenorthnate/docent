use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use crate::GitError;

/// The user's `git`. This module is the only place Docent runs it.
///
/// Git starts from an empty environment plus an allowlist, so forge tokens,
/// askpass helpers and `GIT_CONFIG_*` overrides in Docent's own environment
/// never reach it. It can't prompt: stdin is closed and terminal and
/// credential-manager prompts are off.
#[derive(Debug, Clone)]
pub struct Git {
    env: Vec<(OsString, OsString)>,
}

/// Parent variables passed on to git, so it finds the user's config and
/// speaks their locale. Anything else, tokens included, is dropped.
const ALLOWED: [&str; 7] = [
    "HOME",
    "USER",
    "LOGNAME",
    "TMPDIR",
    "LANG",
    "TZ",
    "XDG_CONFIG_HOME",
];

/// Long enough for any local query; a hung git is killed.
const TIMEOUT: Duration = Duration::from_secs(30);

impl Git {
    /// `path` is the `PATH` to find `git` on, normally the login shell's.
    pub fn new(path: OsString) -> Self {
        Self::with_env(path, std::env::vars_os())
    }

    /// Like [`Git::new`], but filters `parent` instead of this process's
    /// environment. For tests.
    pub fn with_env(
        path: OsString,
        parent: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Self {
        let mut env: Vec<_> = parent
            .into_iter()
            .filter(|(name, _)| {
                name.to_str()
                    .is_some_and(|name| ALLOWED.contains(&name) || name.starts_with("LC_"))
            })
            .collect();
        env.push(("PATH".into(), path));
        env.push(("GIT_TERMINAL_PROMPT".into(), "0".into()));
        env.push(("GCM_INTERACTIVE".into(), "never".into()));
        Self { env }
    }

    /// Runs `git <args>` in `cwd` and returns its stdout. A failure is
    /// logged with git's stderr, and the stderr is returned in the error.
    pub(crate) async fn run(&self, cwd: &Path, args: &[&str]) -> Result<String, GitError> {
        let run = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output();
        let output = match tokio::time::timeout(TIMEOUT, run).await {
            Err(_) => return Err(self.failed(cwd, args, "timed out")),
            Ok(Err(err)) if err.kind() == ErrorKind::NotFound => return Err(GitError::NotFound),
            Ok(Err(err)) => return Err(self.failed(cwd, args, &err.to_string())),
            Ok(Ok(output)) => output,
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(self.failed(cwd, args, stderr.trim()));
        }
        String::from_utf8(output.stdout).map_err(|_| self.failed(cwd, args, "output was not UTF-8"))
    }

    /// Logs a failed invocation (never its environment) and builds the error.
    fn failed(&self, cwd: &Path, args: &[&str], why: &str) -> GitError {
        eprintln!(
            "git {} (in {}) failed: {why}",
            args.join(" "),
            cwd.display()
        );
        GitError::Failed(why.to_string())
    }
}

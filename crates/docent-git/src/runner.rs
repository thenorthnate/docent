use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
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
    /// How long a watched git may print nothing before it is killed.
    stall_limit: Duration,
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
pub(crate) const TIMEOUT: Duration = Duration::from_secs(30);

/// Long enough for a fetch over a slow network.
pub(crate) const NETWORK_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// How long to wait for more of a finished git's stderr.
const DRAIN_LIMIT: Duration = Duration::from_millis(200);

/// How much of a watched git's stderr is kept. The other end of a clone can
/// send any amount of text.
const MAX_LINES: usize = 20;
const MAX_LINE_BYTES: usize = 1000;

impl Git {
    /// `path` is the `PATH` to find `git` on, normally the login shell's.
    pub fn new(path: OsString) -> Self {
        Self::with_env(path, std::env::vars_os())
    }

    /// Like [`Git::new`], but filters `parent` instead of this process's
    /// environment. For tests.
    #[doc(hidden)]
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
        Self {
            env,
            stall_limit: NETWORK_TIMEOUT,
        }
    }

    /// Replaces how long a clone may print nothing. For tests.
    #[doc(hidden)]
    pub fn with_stall_limit(mut self, limit: Duration) -> Self {
        self.stall_limit = limit;
        self
    }

    /// `git <args>` in `cwd`, with the scrubbed environment and no input.
    fn command(&self, cwd: &Path, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .kill_on_drop(true);
        command
    }

    /// Runs `git <args>` in `cwd` and returns its stdout. A failure is
    /// logged with git's stderr, and the stderr is returned in the error.
    pub(crate) async fn run(&self, cwd: &Path, args: &[&str]) -> Result<String, GitError> {
        self.run_for(cwd, args, TIMEOUT).await
    }

    /// Like [`Git::run`], killing git after `timeout` instead.
    pub(crate) async fn run_for(
        &self,
        cwd: &Path,
        args: &[&str],
        timeout: Duration,
    ) -> Result<String, GitError> {
        let stdout = self.run_bytes_for(cwd, args, timeout).await?;
        String::from_utf8(stdout).map_err(|_| self.failed(cwd, args, "output was not UTF-8"))
    }

    /// Like [`Git::run_for`], for output that may not be UTF-8, such as file
    /// names.
    pub(crate) async fn run_bytes_for(
        &self,
        cwd: &Path,
        args: &[&str],
        timeout: Duration,
    ) -> Result<Vec<u8>, GitError> {
        let run = self.command(cwd, args).output();
        let output = match tokio::time::timeout(timeout, run).await {
            Err(_) => return Err(self.failed(cwd, args, "timed out")),
            Ok(Err(err)) if err.kind() == ErrorKind::NotFound => return Err(GitError::NotFound),
            Ok(Err(err)) => return Err(self.failed(cwd, args, &err.to_string())),
            Ok(Ok(output)) => output,
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(self.failed(cwd, args, stderr.trim()));
        }
        Ok(output.stdout)
    }

    /// Runs `git <args>` in `cwd`, for however long it keeps printing to
    /// stderr. Each line goes to `on_line` as it arrives; ones it returns
    /// `true` for are progress, and left out of the error if git fails.
    pub(crate) async fn run_watched(
        &self,
        cwd: &Path,
        args: &[&str],
        mut on_line: impl FnMut(&str) -> bool,
    ) -> Result<(), GitError> {
        let spawned = self
            .command(cwd, args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Err(err) if err.kind() == ErrorKind::NotFound => return Err(GitError::NotFound),
            Err(err) => return Err(self.failed(cwd, args, &err.to_string())),
            Ok(child) => child,
        };
        let mut stderr = child.stderr.take().expect("stderr was piped above");
        let mut lines = Lines::default();
        let mut chunk = [0; 4096];
        let mut status = None;
        loop {
            // Once git has exited, only what it left in the pipe is read: a
            // process it started (say, ssh connection sharing) may keep the
            // pipe open.
            let limit = match status {
                Some(_) => DRAIN_LIMIT,
                None => self.stall_limit,
            };
            let step = tokio::time::timeout(limit, async {
                tokio::select! {
                    read = stderr.read(&mut chunk) => Step::Read(read),
                    exit = child.wait(), if status.is_none() => Step::Exited(exit),
                }
            });
            match step.await {
                Ok(Step::Read(Ok(n))) if n > 0 => lines.push(&chunk[..n], &mut on_line),
                Ok(Step::Read(_)) if status.is_some() => break,
                // The pipe closed, so git is about to exit.
                Ok(Step::Read(_)) => {
                    let exit = tokio::time::timeout(self.stall_limit, child.wait()).await;
                    status = exit.ok();
                    break;
                }
                Ok(Step::Exited(exit)) => status = Some(exit),
                Err(_) => break,
            }
        }
        lines.finish(&mut on_line);
        match status {
            Some(Ok(status)) if status.success() => Ok(()),
            Some(Ok(_)) => Err(self.failed(cwd, args, &lines.text())),
            Some(Err(err)) => Err(self.failed(cwd, args, &err.to_string())),
            None => {
                let _ = child.kill().await;
                Err(self.failed(cwd, args, "it stopped responding"))
            }
        }
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

/// What a watched git did next.
enum Step {
    Read(std::io::Result<usize>),
    Exited(std::io::Result<std::process::ExitStatus>),
}

/// Splits stderr into lines. A progress line ends in a carriage return.
#[derive(Default)]
struct Lines {
    current: Vec<u8>,
    /// The last lines that weren't progress.
    kept: VecDeque<String>,
}

impl Lines {
    fn push(&mut self, bytes: &[u8], on_line: &mut impl FnMut(&str) -> bool) {
        for &byte in bytes {
            if byte == b'\n' || byte == b'\r' {
                self.finish(on_line);
            } else if self.current.len() < MAX_LINE_BYTES {
                self.current.push(byte);
            }
        }
    }

    fn finish(&mut self, on_line: &mut impl FnMut(&str) -> bool) {
        let bytes = std::mem::take(&mut self.current);
        let line = String::from_utf8_lossy(&bytes);
        let line = line.trim();
        if line.is_empty() || on_line(line) {
            return;
        }
        if self.kept.len() == MAX_LINES {
            self.kept.pop_front();
        }
        self.kept.push_back(line.to_string());
    }

    fn text(&self) -> String {
        Vec::from_iter(self.kept.iter().map(String::as_str)).join("\n")
    }
}

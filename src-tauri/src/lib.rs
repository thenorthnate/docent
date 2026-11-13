mod shell_env;
mod watch;

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use docent_forge::{AccountSummary, Accounts, ConnectError, GhCli};
use docent_store::{RepoId, Store, WatchedRepo};
use tauri::{Manager, State};
use tokio::sync::OnceCell;
use watch::CommandError;

/// Process-wide state. Tokens live only in `accounts`, in memory.
///
/// `Default` builds the startup value handed to Tauri's `.manage()`: no
/// accounts yet (pointed at the real GitHub API) and no `PATH` read yet.
#[derive(Default)]
struct AppState {
    accounts: Accounts,
    /// The `PATH` that child processes use, read once from the login shell.
    child_path: OnceCell<OsString>,
}

impl AppState {
    /// `gh` on the login shell's `PATH`. Only a `PATH` the shell actually
    /// reported is kept; after a failure (say, a slow shell) the next
    /// connect asks again.
    async fn gh(&self) -> GhCli {
        if let Some(path) = self.child_path.get() {
            return GhCli::new(path.clone());
        }
        let shell = std::env::var_os("SHELL").map_or_else(|| "/bin/zsh".into(), PathBuf::from);
        match shell_env::login_shell_path(&shell, Duration::from_secs(10)).await {
            Some(path) => GhCli::new(self.child_path.get_or_init(|| async { path }).await.clone()),
            None => GhCli::new(shell_env::fallback_path(std::env::var_os("PATH"))),
        }
    }
}

/// Connects a GitHub host with the token from the user's `gh` login. The
/// token stays in the Rust core; the UI gets the host and login only.
#[tauri::command]
async fn connect_host(
    host: String,
    state: State<'_, AppState>,
) -> Result<AccountSummary, ConnectError> {
    let gh = state.gh().await;
    state.accounts.connect(&host, &gh).await
}

/// The connected hosts, without their tokens.
#[tauri::command]
fn list_accounts(state: State<'_, AppState>) -> Vec<AccountSummary> {
    state.accounts.list()
}

/// Watches `repo` (`owner/name`) on the connected `host`, once the forge
/// confirms it exists.
#[tauri::command]
async fn watch_repo(
    host: String,
    repo: String,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<WatchedRepo, CommandError> {
    watch::watch(&state.accounts, &store, &host, &repo).await
}

/// The watched repos, sorted by host, owner and name.
#[tauri::command]
async fn list_watched_repos(store: State<'_, Store>) -> Result<Vec<WatchedRepo>, CommandError> {
    Ok(store.watched_repos().await?)
}

/// Stops watching a repo.
#[tauri::command]
async fn unwatch_repo(id: RepoId, store: State<'_, Store>) -> Result<(), CommandError> {
    Ok(store.unwatch_repo(id).await?)
}

/// The app version, shown in the UI so a bug report can name it.
///
/// `env!` reads `CARGO_PKG_VERSION` at compile time. Cargo sets it from the
/// workspace `version` in `Cargo.toml`, and the string is baked into the
/// binary, so the user's environment at runtime plays no part.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            // Without its database the app has nothing to show, so a failure
            // here stops startup with the reason.
            let path = app.path().app_data_dir()?.join("docent.sqlite");
            let store = tauri::async_runtime::block_on(Store::open(&path))?;
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            connect_host,
            list_accounts,
            list_watched_repos,
            unwatch_repo,
            watch_repo
        ])
        .run(tauri::generate_context!())
        .expect("Tauri failed to start the app; nothing can run without it");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_matches_the_bundle_version() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json is valid JSON");

        assert_eq!(Some(app_version()), conf["version"].as_str());
    }
}

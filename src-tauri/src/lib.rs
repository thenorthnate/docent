mod command_error;
mod inbox;
mod remember;
mod shell_env;
mod watch;

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use command_error::CommandError;
use docent_forge::{AccountSummary, Accounts, GhCli, Section, SectionPulls};
use docent_store::{RepoId, Store, WatchedRepo};
use tauri::{Manager, State};
use tokio::sync::OnceCell;

/// Process-wide state. Tokens live only in `accounts`, in memory.
///
/// `Default` builds the startup value handed to Tauri's `.manage()`: no
/// accounts yet (pointed at the real GitHub API) and no `PATH` read yet.
#[derive(Default)]
struct AppState {
    accounts: Accounts,
    /// The `PATH` that child processes use, read once from the login shell.
    child_path: OnceCell<OsString>,
    /// Set once the remembered hosts have been reconnected (or have failed).
    restored: OnceCell<()>,
}

impl AppState {
    /// The accounts, once the hosts remembered from the last launch have been
    /// reconnected. Startup begins this in the background; a command that
    /// arrives first waits for it rather than seeing a half-restored list.
    async fn accounts(&self, store: &Store) -> &Accounts {
        self.restored
            .get_or_init(|| async {
                if let Err(err) = remember::restore_all(&self.accounts, store, || self.gh()).await {
                    // Nothing to show this on yet; the user can still connect
                    // by hand in Settings.
                    eprintln!("Could not reconnect remembered hosts: {}", err.message);
                }
            })
            .await;
        &self.accounts
    }

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

/// Connects a GitHub host with the token from the user's `gh` login, and
/// remembers its host name and login. The token stays in the Rust core's
/// memory; the UI and the store get the host and login only.
#[tauri::command]
async fn connect_host(
    host: String,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<AccountSummary, CommandError> {
    let accounts = state.accounts(&store).await;
    let gh = state.gh().await;
    remember::connect(accounts, &store, &host, &gh).await
}

/// The user's hosts, without their tokens. A host that failed to reconnect
/// at startup carries its error.
#[tauri::command]
async fn list_accounts(
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<Vec<AccountSummary>, CommandError> {
    Ok(state.accounts(&store).await.list())
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
    watch::watch(state.accounts(&store).await, &store, &host, &repo).await
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

/// The open PRs in an inbox section (`review_requested`, `assigned`,
/// `authored` or `mentioned`), in watched repos, from every connected host,
/// newest first. Hosts that couldn't be asked are listed with their errors.
#[tauri::command]
async fn list_section(
    section: Section,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<SectionPulls, CommandError> {
    inbox::section_pulls(state.accounts(&store).await, &store, section).await
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
            // Reconnect remembered hosts now, off the main thread, so the
            // window opens without waiting on `gh` or the network.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                state.accounts(&handle.state::<Store>()).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            connect_host,
            list_accounts,
            list_section,
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

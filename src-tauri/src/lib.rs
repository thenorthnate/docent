mod bind;
mod changed_files;
mod clone;
mod closed;
mod command_error;
mod evict;
mod inbox;
mod remember;
mod review;
mod shell_env;
mod watch;
mod worktree;
mod worktrees;

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use clone::CloneUpdate;
use command_error::CommandError;
use docent_forge::{AccountSummary, Accounts, GhCli, PullOverview, Section, SectionPulls};
use docent_git::{ChangedFile, FileDiff, Git};
use docent_store::{RepoId, Store, WatchedRepo};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::OnceCell;
use worktrees::{InUse, Worktrees};

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
    worktrees_in_use: InUse,
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

    /// `gh` on the login shell's `PATH`.
    async fn gh(&self) -> GhCli {
        GhCli::new(self.child_path().await)
    }

    /// `git` on the login shell's `PATH`, with a scrubbed environment.
    async fn git(&self) -> Git {
        Git::new(self.child_path().await)
    }

    /// The login shell's `PATH`, for child processes. Only a `PATH` the
    /// shell actually reported is kept; after a failure (say, a slow shell)
    /// the next call asks again.
    async fn child_path(&self) -> OsString {
        if let Some(path) = self.child_path.get() {
            return path.clone();
        }
        let shell = std::env::var_os("SHELL").map_or_else(|| "/bin/zsh".into(), PathBuf::from);
        match shell_env::login_shell_path(&shell, Duration::from_secs(10)).await {
            Some(path) => self.child_path.get_or_init(|| async { path }).await.clone(),
            None => shell_env::fallback_path(std::env::var_os("PATH")),
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

/// Asks for a folder with the native picker. `None` means the user cancelled.
async fn pick_folder(app: &AppHandle, title: String) -> Result<Option<PathBuf>, CommandError> {
    let (picked, chosen) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
        .pick_folder(move |folder| {
            let _ = picked.send(folder);
        });
    // A dropped sender (the dialog closed without answering) is a cancel.
    let Some(folder) = chosen.await.ok().flatten() else {
        return Ok(None);
    };
    folder.into_path().map(Some).map_err(|err| CommandError {
        kind: "invalid_path",
        message: err.to_string(),
    })
}

/// Asks for a folder with the native picker, then binds watched repo `id`
/// to the clone in it. Returns `None` if the user cancels. The folder comes
/// from the picker, never from the UI.
#[tauri::command]
async fn bind_repo(
    id: RepoId,
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<Option<WatchedRepo>, CommandError> {
    let repo = store.watched_repo(id).await?;
    let title = format!("Choose your clone of {}/{}", repo.owner, repo.name);
    let Some(folder) = pick_folder(&app, title).await? else {
        return Ok(None);
    };
    bind::bind(&store, &state.git().await, &repo, &folder)
        .await
        .map(Some)
}

/// Asks for a folder with the native picker, then clones watched repo `id`
/// into it and binds the clone. Returns `None` if the user cancels. Reports
/// to `on_progress` once a folder is picked.
#[tauri::command]
async fn clone_repo(
    id: RepoId,
    on_progress: Channel<CloneUpdate>,
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<Option<WatchedRepo>, CommandError> {
    let repo = store.watched_repo(id).await?;
    let title = format!("Choose where to clone {}/{}", repo.owner, repo.name);
    let Some(parent) = pick_folder(&app, title).await? else {
        return Ok(None);
    };
    clone::clone(&store, &state.git().await, &repo, &parent, |update| {
        // The window may have gone; the clone carries on without it.
        let _ = on_progress.send(update);
    })
    .await
    .map(Some)
}

/// Opens PR `number` of `owner/repo` on `host` in its own worktree, under
/// Docent's data folder. An unbound repo comes back as `unbound`, for the UI
/// to offer Bind or Clone.
#[tauri::command]
async fn open_pr_worktree(
    host: String,
    owner: String,
    repo: String,
    number: u64,
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<worktree::OpenedPr, CommandError> {
    let worktrees = worktrees(&app, &state, &store).await?;
    // The eviction carries on in the background.
    let (opened, _evicting) = worktrees.open(&host, &owner, &repo, number).await;
    opened
}

/// The files a PR changes, from its merge base to its head, read from its
/// open worktree.
#[tauri::command]
async fn changed_files(
    range: changed_files::PullRange,
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<Vec<ChangedFile>, CommandError> {
    let worktrees = worktrees(&app, &state, &store).await?;
    // The eviction carries on in the background.
    let (files, _evicting) = worktrees.changed_files(&range).await;
    files
}

/// One changed file's contents and git's hunks, read from its PR's open
/// worktree. `blobs` are the file's, as `changed_files` listed them.
#[tauri::command]
async fn file_diff(
    range: changed_files::PullRange,
    blobs: changed_files::FileBlobs,
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<FileDiff, CommandError> {
    let worktrees = worktrees(&app, &state, &store).await?;
    // The eviction carries on in the background.
    let (diff, _evicting) = worktrees.file_diff(&range, &blobs).await;
    diff
}

/// The folder in Docent's data folder that holds every PR worktree, and what
/// working in it needs.
async fn worktrees(
    app: &AppHandle,
    state: &AppState,
    store: &Store,
) -> Result<Worktrees, CommandError> {
    let data = app.path().app_data_dir().map_err(|err| CommandError {
        kind: "invalid_path",
        message: format!("Docent couldn't find its data folder: {err}"),
    })?;
    Ok(Worktrees {
        in_use: state.worktrees_in_use.clone(),
        store: store.clone(),
        git: state.git().await,
        root: data.join("worktrees"),
    })
}

/// Removes the worktrees and pins of PRs that closed or merged. The inbox
/// runs it on each refresh.
#[tauri::command]
async fn remove_closed_worktrees(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<(), CommandError> {
    let accounts = state.accounts(&store).await;
    worktrees(&app, &state, &store)
        .await?
        .remove_done(accounts)
        .await;
    Ok(())
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

/// The description, checks and timeline of PR `number` in `owner/repo` on
/// `host`, for the review page's Overview tab. Watched repos only.
#[tauri::command]
async fn pull_overview(
    host: String,
    owner: String,
    repo: String,
    number: u64,
    state: State<'_, AppState>,
    store: State<'_, Store>,
) -> Result<PullOverview, CommandError> {
    let accounts = state.accounts(&store).await;
    review::pull_overview(accounts, &store, &host, &owner, &repo, number).await
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
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            // Without its database the app has nothing to show, so a failure
            // here stops startup with the reason.
            let path = app.path().app_data_dir()?.join("docent.sqlite");
            let store = tauri::async_runtime::block_on(Store::open(&path))?;
            app.manage(store);
            // Reconnect remembered hosts now, off the main thread, so the
            // window opens without waiting on `gh` or the network. Then
            // remove the worktrees of PRs that closed since the last launch.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                let store = handle.state::<Store>();
                let accounts = state.accounts(&store).await;
                match worktrees(&handle, &state, &store).await {
                    Ok(worktrees) => worktrees.remove_done(accounts).await,
                    Err(err) => eprintln!("Could not check for closed PRs: {}", err.message),
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            bind_repo,
            changed_files,
            clone_repo,
            connect_host,
            file_diff,
            list_accounts,
            list_section,
            list_watched_repos,
            open_pr_worktree,
            pull_overview,
            remove_closed_worktrees,
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

    /// One CSP directive's sources from `tauri.conf.json`'s `key` (`csp` or
    /// `devCsp`).
    fn csp_sources(key: &str, directive: &str) -> Vec<String> {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json is valid JSON");
        let sources = conf["app"]["security"][key][directive]
            .as_str()
            .unwrap_or_else(|| panic!("{key} sets {directive}"));
        sources.split_whitespace().map(str::to_string).collect()
    }

    /// PR bodies, comments and agent output are hostile (PLAN §5.6), so no
    /// script may come from anywhere but the app's own bundle: no inline
    /// script, no eval, no remote host. Tauri adds hashes for its own
    /// injected scripts at build time.
    #[test]
    fn the_csp_runs_only_the_apps_own_scripts() {
        for key in ["csp", "devCsp"] {
            assert_eq!(csp_sources(key, "default-src"), ["'self'"], "{key}");
            assert_eq!(csp_sources(key, "script-src"), ["'self'"], "{key}");
            for directive in ["object-src", "frame-src", "base-uri", "form-action"] {
                assert_eq!(csp_sources(key, directive), ["'none'"], "{key} {directive}");
            }
        }
    }
}

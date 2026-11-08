//! Docent M0.5 work-laptop proof. Throwaway spike code: see spike/README.md.

mod acp;
mod gh;
mod git;

use tokio::sync::{oneshot, Mutex};

/// Environment variables that carry forge tokens. Stripped from every child process.
pub const TOKEN_VARS: &[&str] = &[
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
];

#[derive(Default)]
pub struct AppState {
    pub gh: Mutex<Option<gh::Session>>,
    pub cancel: Mutex<Option<oneshot::Sender<()>>>,
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            gh::connect,
            gh::probe_cmd,
            gh::list_prs,
            gh::prs_for_commit,
            gh::pending_review_roundtrip,
            git::checkout_pr_cmd,
            git::hunk_context,
            acp::run_agent,
            acp::cancel_agent,
        ])
        .run(tauri::generate_context!())
        .expect("tauri app failed to start");
}

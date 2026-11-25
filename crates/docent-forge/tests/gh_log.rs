//! What is logged when `gh` gives no token. The log is the process's stderr,
//! which a test can't read from inside, so each case reruns this test binary
//! as a child and reads the child's output.
#![cfg(unix)]

mod common;

use std::process::Command;

use common::FakeGh;
use docent_forge::{Accounts, ConnectError};

const SCRIPT: &str = "DOCENT_TEST_GH_SCRIPT";

/// Not a test by itself: the child that `failed_connect` runs.
#[tokio::test]
#[ignore = "run as a child by the other tests"]
async fn child() {
    let script = std::env::var(SCRIPT).expect("set by failed_connect");
    let gh = FakeGh::with_script(&script);
    let accounts = Accounts::with_api_base(|_| unreachable!("no API call without a token"));

    let err = accounts.connect("github.com", &gh.cli()).await.unwrap_err();

    assert_eq!(err, ConnectError::NotLoggedIn("github.com".to_string()));
    println!("error: {err} {err:?} {accounts:?}");
}

/// Connects with a fake `gh` running `script`, and returns everything the
/// child logged or put in its error.
fn failed_connect(script: &str) -> String {
    let output = Command::new(std::env::current_exe().expect("test binary"))
        .args(["child", "--exact", "--ignored", "--nocapture"])
        .env(SCRIPT, script)
        .output()
        .expect("rerun test binary");
    let shown = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("error: "), "the child didn't run: {shown}");
    shown
}

#[test]
fn a_failed_gh_logs_its_exit_status() {
    let shown = failed_connect("exit 7\n");

    assert!(shown.contains("exit status: 7"), "{shown}");
}

#[test]
fn a_killed_gh_logs_the_signal() {
    let shown = failed_connect("kill -KILL $$\n");

    assert!(shown.contains("signal: 9"), "{shown}");
}

// ---- Denial test: anything gh prints could hold a token. ----

#[test]
fn nothing_gh_prints_reaches_the_log_or_the_error() {
    let print_both = "printf 'gho_stdout'; printf 'gho_stderr' >&2";
    for (script, status) in [
        (format!("{print_both}\nexit 1\n"), "exit status: 1"),
        (format!("{print_both}\nkill -KILL $$\n"), "signal: 9"),
        // Success without a usable token: none at all, then not UTF-8.
        (
            "printf 'gho_stderr' >&2\nexit 0\n".to_string(),
            "exit status: 0",
        ),
        (
            format!("printf '\\377'; {print_both}\nexit 0\n"),
            "exit status: 0",
        ),
    ] {
        let shown = failed_connect(&script);

        assert!(shown.contains(status), "{script}: {shown}");
        for secret in ["gho_stdout", "gho_stderr"] {
            assert!(!shown.contains(secret), "{script} leaked: {shown}");
        }
    }
}

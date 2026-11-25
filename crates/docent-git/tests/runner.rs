//! Denial tests for the git runner: whatever the app's environment holds,
//! git sees only an allowlist, and it can't prompt for input.
#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use docent_git::Git;

/// A `git` that writes its environment, and whether stdin is a terminal or
/// has input, to `dump`, then fails.
fn fake_git(dir: &Path, dump: &Path) {
    let script = format!(
        "#!/bin/sh\n\
         case \"$*\" in ls-remote*) echo https://github.com/octocat/Hello-World.git; exit 0;; esac\n\
         env > '{dump}'\n\
         if [ -t 0 ]; then echo STDIN_IS_TTY >> '{dump}'; fi\n\
         if read -r line; then echo STDIN_HAD_INPUT >> '{dump}'; fi\n\
         exit 1\n",
        dump = dump.display()
    );
    let git = dir.join("git");
    std::fs::write(&git, script).expect("write fake git");
    std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

fn var(name: &str, value: &str) -> (OsString, OsString) {
    (name.into(), value.into())
}

async fn env_seen_by_git(parent: Vec<(OsString, OsString)>) -> String {
    let bin = tempfile::tempdir().expect("temp dir");
    let dump = bin.path().join("env.txt");
    fake_git(bin.path(), &dump);
    // /bin for the fake script's own `env`.
    let path = std::env::join_paths([bin.path(), Path::new("/usr/bin"), Path::new("/bin")])
        .expect("join PATH");
    let git = Git::with_env(path, parent);

    let result = git
        .check_clone(bin.path(), "github.com", "octocat", "Hello-World")
        .await;

    assert!(result.is_err(), "the fake git always fails");
    std::fs::read_to_string(&dump).expect("fake git ran and wrote its environment")
}

fn names(env: &str) -> Vec<&str> {
    env.lines()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name))
        .collect()
}

#[tokio::test]
async fn forge_tokens_and_credential_helpers_never_reach_git() {
    let env = env_seen_by_git(vec![
        var("GH_TOKEN", "gho_secret1"),
        var("GITHUB_TOKEN", "gho_secret2"),
        var("GH_ENTERPRISE_TOKEN", "gho_secret3"),
        var("GITHUB_ENTERPRISE_TOKEN", "gho_secret4"),
        var("GIT_ASKPASS", "/tmp/askpass"),
        var("SSH_ASKPASS", "/tmp/askpass"),
        var("GIT_CONFIG_PARAMETERS", "'credential.helper'='!evil'"),
        var("GIT_CONFIG_COUNT", "1"),
        var("SOME_OTHER_SECRET", "s3cret"),
        var("HOME", "/Users/someone"),
    ])
    .await;

    assert!(!env.contains("gho_secret"), "a token reached git:\n{env}");
    assert!(
        !env.contains("s3cret"),
        "a non-allowlisted variable reached git:\n{env}"
    );
    for name in names(&env) {
        assert!(
            !name.contains("TOKEN") && !name.contains("ASKPASS") && !name.starts_with("GIT_CONFIG"),
            "{name} reached git:\n{env}"
        );
    }
    // The allowlist still gets through, so git finds the user's config.
    assert!(env.lines().any(|l| l == "HOME=/Users/someone"), "{env}");
}

/// Nothing from this test process's own environment leaks through either:
/// the runner starts git from an empty environment, not a filtered copy.
#[tokio::test]
async fn new_starts_git_from_a_cleared_environment() {
    let bin = tempfile::tempdir().expect("temp dir");
    let dump = bin.path().join("env.txt");
    fake_git(bin.path(), &dump);
    let path = std::env::join_paths([bin.path(), Path::new("/usr/bin"), Path::new("/bin")])
        .expect("join PATH");

    let _ = Git::new(path)
        .check_clone(bin.path(), "github.com", "octocat", "Hello-World")
        .await;

    let env = std::fs::read_to_string(&dump).expect("fake git ran");
    // Cargo sets these for every test process.
    assert!(
        !env.contains("CARGO_"),
        "the test process's environment leaked:\n{env}"
    );
}

#[tokio::test]
async fn git_cannot_prompt_for_input() {
    let env = env_seen_by_git(vec![]).await;

    assert!(env.lines().any(|l| l == "GIT_TERMINAL_PROMPT=0"), "{env}");
    assert!(env.lines().any(|l| l == "GCM_INTERACTIVE=never"), "{env}");
    assert!(!env.contains("STDIN_IS_TTY"), "{env}");
    assert!(!env.contains("STDIN_HAD_INPUT"), "{env}");
}

/// Cloning talks to the forge, so it is where a token would be most useful to
/// git. It uses the user's own git credentials instead.
#[tokio::test]
async fn forge_tokens_never_reach_a_clone() {
    let bin = tempfile::tempdir().expect("temp dir");
    let dump = bin.path().join("env.txt");
    fake_git(bin.path(), &dump);
    let path = std::env::join_paths([bin.path(), Path::new("/usr/bin"), Path::new("/bin")])
        .expect("join PATH");
    let parent = tempfile::tempdir().expect("temp dir");

    let result = Git::with_env(
        path,
        [
            var("GH_TOKEN", "gho_secret1"),
            var("GITHUB_TOKEN", "gho_secret2"),
            var("GIT_ASKPASS", "/tmp/askpass"),
            var("HOME", "/Users/someone"),
        ],
    )
    .clone_repo(
        parent.path(),
        "github.com",
        "octocat",
        "Hello-World",
        |_| {},
    )
    .await;

    assert!(result.is_err(), "the fake git always fails");
    let env = std::fs::read_to_string(&dump).expect("fake git ran");
    for leaked in ["gho_secret", "TOKEN", "ASKPASS"] {
        assert!(!env.contains(leaked), "{leaked} reached git:\n{env}");
    }
    assert!(env.lines().any(|l| l == "GIT_TERMINAL_PROMPT=0"), "{env}");
    assert!(!env.contains("STDIN_IS_TTY"), "{env}");
    assert!(!env.contains("STDIN_HAD_INPUT"), "{env}");
}

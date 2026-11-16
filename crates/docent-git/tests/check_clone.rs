//! `Git::check_clone` against temp repos built in the test.

use std::path::{Path, PathBuf};
use std::process::Command;

use docent_git::{Git, GitError};

fn git() -> Git {
    Git::new(std::env::var_os("PATH").expect("tests run with a PATH"))
}

/// Runs the test machine's git directly, to build fixtures.
fn sh_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// A fresh repo with the given remotes.
fn repo_with_remotes(remotes: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    sh_git(dir.path(), &["init", "--quiet"]);
    for (name, url) in remotes {
        sh_git(dir.path(), &["remote", "add", name, url]);
    }
    dir
}

fn real(path: &Path) -> PathBuf {
    path.canonicalize().expect("canonicalize")
}

#[tokio::test]
async fn a_clone_whose_origin_is_the_repo_matches() {
    let repo = repo_with_remotes(&[("origin", "https://github.com/octocat/Hello-World.git")]);

    let clone = git()
        .check_clone(repo.path(), "github.com", "octocat", "Hello-World")
        .await
        .expect("matches");

    assert_eq!(real(&clone.path), real(repo.path()));
}

#[tokio::test]
async fn any_remote_can_match_and_case_is_ignored() {
    let repo = repo_with_remotes(&[
        ("origin", "git@github.com:me/hello-world-fork.git"),
        ("upstream", "git@github.com:OctoCat/hello-world.git"),
    ]);

    let clone = git()
        .check_clone(repo.path(), "github.com", "octocat", "Hello-World")
        .await;

    assert!(clone.is_ok(), "{clone:?}");
}

#[tokio::test]
async fn a_subfolder_binds_the_clone_top_level() {
    let repo = repo_with_remotes(&[("origin", "https://github.com/octocat/Hello-World")]);
    let sub = repo.path().join("src/deep");
    std::fs::create_dir_all(&sub).expect("mkdir");

    let clone = git()
        .check_clone(&sub, "github.com", "octocat", "Hello-World")
        .await
        .expect("matches");

    assert_eq!(real(&clone.path), real(repo.path()));
}

#[tokio::test]
async fn a_folder_that_is_not_a_git_repo_is_rejected() {
    let dir = tempfile::tempdir().expect("temp dir");

    let err = git()
        .check_clone(dir.path(), "github.com", "octocat", "Hello-World")
        .await
        .unwrap_err();

    assert!(matches!(err, GitError::NotARepo(_)), "{err:?}");
    assert!(err.to_string().contains("isn't a git clone"), "{err}");
}

#[tokio::test]
async fn a_missing_folder_is_rejected_as_not_a_repo() {
    let dir = tempfile::tempdir().expect("temp dir");

    let err = git()
        .check_clone(
            &dir.path().join("gone"),
            "github.com",
            "octocat",
            "Hello-World",
        )
        .await
        .unwrap_err();

    assert!(matches!(err, GitError::NotARepo(_)), "{err:?}");
}

#[tokio::test]
async fn a_clone_of_another_repo_is_rejected_listing_its_remotes() {
    let repo = repo_with_remotes(&[
        ("origin", "https://github.com/octocat/Spoon-Knife.git"),
        ("work", "git@ghe.example.com:octocat/Hello-World.git"),
    ]);

    let err = git()
        .check_clone(repo.path(), "github.com", "octocat", "Hello-World")
        .await
        .unwrap_err();

    assert_eq!(err.kind(), "no_matching_remote");
    let message = err.to_string();
    assert!(
        message.contains("github.com:octocat/Hello-World"),
        "{message}"
    );
    assert!(
        message.contains("https://github.com/octocat/Spoon-Knife.git"),
        "{message}"
    );
    assert!(
        message.contains("git@ghe.example.com:octocat/Hello-World.git"),
        "{message}"
    );
}

#[tokio::test]
async fn a_repo_with_no_remotes_is_rejected() {
    let repo = repo_with_remotes(&[]);

    let err = git()
        .check_clone(repo.path(), "github.com", "octocat", "Hello-World")
        .await
        .unwrap_err();

    assert_eq!(err.kind(), "no_matching_remote");
    assert!(err.to_string().contains("no remotes"), "{err}");
}

/// The error reaches the UI, so a password in a remote URL is left out.
#[tokio::test]
async fn credentials_in_remote_urls_are_not_repeated() {
    let repo = repo_with_remotes(&[(
        "origin",
        "https://me:gho_s3cret@github.com/octocat/Spoon-Knife.git",
    )]);

    let err = git()
        .check_clone(repo.path(), "github.com", "octocat", "Hello-World")
        .await
        .unwrap_err();

    let message = err.to_string();
    assert!(!message.contains("gho_s3cret"), "{message}");
    assert!(
        message.contains("github.com/octocat/Spoon-Knife.git"),
        "{message}"
    );
}

/// The folder isn't trusted yet: checking it must not run anything its repo
/// config names, such as an fsmonitor hook.
#[cfg(unix)]
#[tokio::test]
async fn checking_a_clone_runs_none_of_its_configured_commands() {
    use std::os::unix::fs::PermissionsExt;

    let repo = repo_with_remotes(&[("origin", "https://github.com/octocat/Hello-World.git")]);
    let marker = repo.path().join("PWNED");
    let hook = repo.path().join(".git/evil.sh");
    std::fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display()))
        .expect("write hook");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    sh_git(
        repo.path(),
        &["config", "core.fsmonitor", &hook.display().to_string()],
    );
    std::fs::write(
        repo.path().join(".git/hooks/post-checkout"),
        "#!/bin/sh\ntouch PWNED\n",
    )
    .expect("hook");

    git()
        .check_clone(repo.path(), "github.com", "octocat", "Hello-World")
        .await
        .expect("matches");

    assert!(!marker.exists(), "a repo-configured command ran");
}

/// Only "not a git repository" means the folder isn't a clone. Other
/// failures, such as git refusing a clone it considers unsafe, keep git's
/// own message so the user can act on it.
#[cfg(unix)]
#[tokio::test]
async fn other_git_failures_keep_gits_message() {
    use std::os::unix::fs::PermissionsExt;

    let bin = tempfile::tempdir().expect("temp dir");
    let fake = bin.path().join("git");
    std::fs::write(
        &fake,
        "#!/bin/sh\necho \"fatal: detected dubious ownership in repository at '/x'\" >&2\nexit 128\n",
    )
    .expect("write fake git");
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let err = Git::new(bin.path().as_os_str().to_owned())
        .check_clone(bin.path(), "github.com", "octocat", "Hello-World")
        .await
        .unwrap_err();

    assert_eq!(err.kind(), "git_failed", "{err:?}");
    assert!(err.to_string().contains("dubious ownership"), "{err}");
}

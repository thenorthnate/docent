//! `Git::open_pr_worktree` against temp repos: a bare repo stands in for
//! the forge, and a clone of it for the user's clone.

use std::path::{Path, PathBuf};
use std::process::Command;

use docent_git::Git;

fn git() -> Git {
    Git::new(std::env::var_os("PATH").expect("tests run with a PATH"))
}

/// Runs the test machine's git directly, to build fixtures, and returns its
/// trimmed stdout.
fn sh_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .trim()
        .to_string()
}

/// A forge, a clone of it, and a scratch repo for making PR commits.
struct Fixture {
    _root: tempfile::TempDir,
    forge: PathBuf,
    clone: PathBuf,
    work: PathBuf,
    /// Where Docent keeps its worktrees.
    data: PathBuf,
}

impl Fixture {
    /// The forge has `main` with one commit, and the clone has it checked out.
    fn new() -> Self {
        let root = tempfile::tempdir().expect("temp dir");
        let base = root.path().canonicalize().expect("canonicalize");
        let forge = base.join("forge.git");
        let work = base.join("work");
        let clone = base.join("clone");
        let data = base.join("data");
        sh_git(
            &base,
            &["init", "--quiet", "--bare", "-b", "main", "forge.git"],
        );
        sh_git(&base, &["init", "--quiet", "-b", "main", "work"]);
        sh_git(&work, &["remote", "add", "origin", forge.to_str().unwrap()]);
        let fixture = Self {
            _root: root,
            forge,
            clone,
            work,
            data,
        };
        fixture.commit("README", "hello\n");
        sh_git(&fixture.work, &["push", "--quiet", "origin", "main"]);
        sh_git(
            &base,
            &["clone", "--quiet", fixture.forge.to_str().unwrap(), "clone"],
        );
        fixture
    }

    /// Commits `file` with `contents` in the scratch repo; returns the SHA.
    fn commit(&self, file: &str, contents: &str) -> String {
        std::fs::write(self.work.join(file), contents).expect("write");
        sh_git(&self.work, &["add", file]);
        sh_git(&self.work, &["commit", "--quiet", "-m", file]);
        sh_git(&self.work, &["rev-parse", "HEAD"])
    }

    /// Points the forge's `refs/pull/<number>/head` at `sha`, as GitHub does
    /// for every PR, fork or not. `--force` allows a force-push.
    fn publish_pr(&self, number: u64, sha: &str) {
        sh_git(
            &self.work,
            &[
                "push",
                "--quiet",
                "--force",
                "origin",
                &format!("{sha}:refs/pull/{number}/head"),
            ],
        );
    }

    fn worktree(&self, number: u64) -> PathBuf {
        self.data.join(format!("pr-{number}"))
    }

    /// Everything about the clone a user would notice changing.
    fn user_state(&self) -> String {
        [
            sh_git(&self.clone, &["symbolic-ref", "HEAD"]),
            sh_git(&self.clone, &["rev-parse", "HEAD"]),
            sh_git(
                &self.clone,
                &["for-each-ref", "refs/heads", "refs/remotes", "refs/tags"],
            ),
            sh_git(&self.clone, &["status", "--porcelain", "--untracked=all"]),
        ]
        .join("\n")
    }
}

/// The head is on no forge branch, only `refs/pull/7/head`: how GitHub
/// shows a fork PR in the base repo.
#[tokio::test]
async fn opening_a_pr_creates_a_detached_worktree_at_its_head() {
    let f = Fixture::new();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    let before = f.user_state();

    let opened = git()
        .open_pr_worktree(&f.clone, "origin", 7, &f.worktree(7))
        .await
        .expect("opens");

    assert_eq!(opened.head_sha, head);
    assert_eq!(opened.path, f.worktree(7));
    assert_eq!(sh_git(&opened.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(opened.path.join("feature.txt")).unwrap(),
        "new\n"
    );
    // Detached: HEAD names no branch.
    let symbolic = Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .current_dir(&opened.path)
        .status()
        .unwrap();
    assert!(!symbolic.success(), "the worktree is on a branch");
    assert_eq!(f.user_state(), before, "the user's clone changed");
}

#[tokio::test]
async fn reopening_after_a_force_push_moves_the_worktree_and_keeps_the_old_head() {
    let f = Fixture::new();
    let first = f.commit("feature.txt", "first\n");
    f.publish_pr(7, &first);
    git()
        .open_pr_worktree(&f.clone, "origin", 7, &f.worktree(7))
        .await
        .expect("opens");
    // Rewrite history: the new head doesn't contain the old one.
    sh_git(&f.work, &["reset", "--quiet", "--hard", "HEAD~1"]);
    let rewritten = f.commit("feature.txt", "rewritten\n");
    f.publish_pr(7, &rewritten);
    let before = f.user_state();

    let reopened = git()
        .open_pr_worktree(&f.clone, "origin", 7, &f.worktree(7))
        .await
        .expect("reopens");

    assert_eq!(reopened.head_sha, rewritten);
    assert_eq!(reopened.path, f.worktree(7));
    assert_eq!(sh_git(&reopened.path, &["rev-parse", "HEAD"]), rewritten);
    assert_eq!(
        std::fs::read_to_string(reopened.path.join("feature.txt")).unwrap(),
        "rewritten\n"
    );
    // The old head is still reachable from a ref, so gc keeps it for
    // interdiffs.
    let pinning = sh_git(
        &f.clone,
        &[
            "for-each-ref",
            "--contains",
            &first,
            "--format=%(refname)",
            "refs/docent",
        ],
    );
    assert!(!pinning.is_empty(), "no refs/docent ref keeps {first}");
    assert_eq!(f.user_state(), before, "the user's clone changed");
}

#[tokio::test]
async fn reopening_an_unchanged_pr_reuses_its_worktree() {
    let f = Fixture::new();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    let git = git();
    let dest = f.worktree(7);
    git.open_pr_worktree(&f.clone, "origin", 7, &dest)
        .await
        .expect("opens");

    let reopened = git
        .open_pr_worktree(&f.clone, "origin", 7, &dest)
        .await
        .expect("reopens");

    assert_eq!(reopened.head_sha, head);
    let worktrees = sh_git(&f.clone, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 2, "{worktrees}");
}

#[tokio::test]
async fn a_deleted_worktree_folder_is_recreated() {
    let f = Fixture::new();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    let git = git();
    let dest = f.worktree(7);
    git.open_pr_worktree(&f.clone, "origin", 7, &dest)
        .await
        .expect("opens");
    std::fs::remove_dir_all(&dest).expect("delete the worktree folder");

    let reopened = git
        .open_pr_worktree(&f.clone, "origin", 7, &dest)
        .await
        .expect("reopens");

    assert_eq!(sh_git(&reopened.path, &["rev-parse", "HEAD"]), head);
}

/// PR content is hostile and the clone's config isn't Docent's: opening a PR
/// must not run any hook or fsmonitor the clone names.
#[cfg(unix)]
#[tokio::test]
async fn opening_a_pr_runs_none_of_the_clones_configured_commands() {
    use std::os::unix::fs::PermissionsExt;

    let f = Fixture::new();
    let marker = f.data.parent().unwrap().join("PWNED");
    let script = f.clone.join(".git/evil.sh");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .expect("write script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    for hook in ["post-checkout", "reference-transaction", "pre-auto-gc"] {
        let path = f.clone.join(".git/hooks").join(hook);
        std::fs::copy(&script, &path).expect("install hook");
    }
    sh_git(
        &f.clone,
        &["config", "core.fsmonitor", script.to_str().unwrap()],
    );
    sh_git(&f.clone, &["config", "gc.auto", "1"]);
    let first = f.commit("feature.txt", "first\n");
    f.publish_pr(7, &first);
    let git = git();
    git.open_pr_worktree(&f.clone, "origin", 7, &f.worktree(7))
        .await
        .expect("opens");
    // The re-open path checks out inside the worktree.
    let second = f.commit("more.txt", "more\n");
    f.publish_pr(7, &second);

    git.open_pr_worktree(&f.clone, "origin", 7, &f.worktree(7))
        .await
        .expect("reopens");

    assert!(!marker.exists(), "a clone-configured command ran");
}

/// Many users map PR heads into their remote-tracking refs. Docent's fetch
/// must still write only under `refs/docent/*`.
#[tokio::test]
async fn a_configured_pull_refspec_does_not_touch_the_users_refs() {
    let f = Fixture::new();
    sh_git(
        &f.clone,
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            "+refs/pull/*/head:refs/remotes/origin/pr/*",
        ],
    );
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    let before = f.user_state();

    git()
        .open_pr_worktree(&f.clone, "origin", 7, &f.worktree(7))
        .await
        .expect("opens");

    assert_eq!(f.user_state(), before, "the user's clone changed");
}

/// The PR's author, or anyone who checked the PR out, may have its head on a
/// local branch, even with uncommitted edits. The worktree is detached, so git
/// allows the same commit there, and the user's branch and edits stay put.
#[tokio::test]
async fn a_pr_head_already_checked_out_on_a_user_branch_still_opens() {
    let f = Fixture::new();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    sh_git(
        &f.clone,
        &["fetch", "--quiet", "origin", "refs/pull/7/head:feature"],
    );
    sh_git(&f.clone, &["switch", "--quiet", "feature"]);
    std::fs::write(f.clone.join("feature.txt"), "my edit\n").expect("edit");
    let before = f.user_state();

    let opened = git()
        .open_pr_worktree(&f.clone, "origin", 7, &f.worktree(7))
        .await
        .expect("opens");

    assert_eq!(sh_git(&opened.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(f.user_state(), before, "the user's branch or edits changed");
    assert_eq!(
        std::fs::read_to_string(f.clone.join("feature.txt")).unwrap(),
        "my edit\n"
    );
}

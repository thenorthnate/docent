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
        sh_git(&base, &["init", "--quiet", "--bare", "-b", "main", "forge.git"]);
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

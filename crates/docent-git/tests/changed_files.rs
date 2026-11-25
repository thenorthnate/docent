//! `Git::changed_files` against temp repos: a bare repo stands in for the
//! forge, and a clone of it for the user's clone.

use std::path::{Path, PathBuf};
use std::process::Command;

use docent_git::{
    ChangedFile, FileStatus, Git, GitError, LineCounts, PrWorktreePath, WorktreesRoot,
};

fn git() -> Git {
    Git::new(std::env::var_os("PATH").expect("tests run with a PATH"))
}

/// Runs the test machine's git directly and returns its trimmed stdout.
fn sh_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// A forge, a clone of it, and a scratch repo for making commits.
struct Fixture {
    root: tempfile::TempDir,
    clone: PathBuf,
    work: PathBuf,
}

impl Fixture {
    /// The forge's `main` has `files`, and the clone has fetched it.
    fn new(files: &[(&str, &[u8])]) -> Self {
        let root = tempfile::tempdir().expect("temp dir");
        let base = root.path().canonicalize().expect("canonicalize");
        let forge = base.join("forge.git");
        let forge = forge.to_str().unwrap();
        sh_git(&base, &["init", "--quiet", "--bare", "-b", "main", forge]);
        sh_git(&base, &["init", "--quiet", "-b", "main", "work"]);
        let (clone, work) = (base.join("clone"), base.join("work"));
        sh_git(&work, &["remote", "add", "origin", forge]);
        let fixture = Self { root, clone, work };
        for (name, contents) in files {
            fixture.write(name, contents);
        }
        fixture.commit();
        sh_git(&fixture.work, &["push", "--quiet", "origin", "main"]);
        sh_git(&base, &["clone", "--quiet", forge, "clone"]);
        fixture
    }

    fn write(&self, name: &str, contents: &[u8]) {
        let path = self.work.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
        std::fs::write(path, contents).expect("write");
    }

    /// Commits everything in the scratch repo; returns the SHA.
    fn commit(&self) -> String {
        sh_git(&self.work, &["add", "--all"]);
        sh_git(&self.work, &["commit", "--quiet", "--allow-empty", "-m", "x"]);
        sh_git(&self.work, &["rev-parse", "HEAD"])
    }

    fn dest(&self) -> PrWorktreePath {
        WorktreesRoot::new(self.root.path().join("data")).pr(1, 7)
    }

    /// Commits the scratch repo's changes as PR 7 and opens it; returns its
    /// head.
    async fn open_pr(&self) -> String {
        let head = self.commit();
        let refspec = format!("{head}:refs/pull/7/head");
        sh_git(&self.work, &["push", "--quiet", "origin", &refspec]);
        git()
            .open_pr_worktree(&self.clone, "origin", &self.dest())
            .await
            .expect("opens");
        head
    }

    /// `main` as the clone last fetched it.
    fn main(&self) -> String {
        sh_git(&self.clone, &["rev-parse", "origin/main"])
    }

    async fn changed(&self, base: &str, head: &str) -> Result<Vec<ChangedFile>, GitError> {
        git().changed_files("origin", &self.dest(), base, head).await
    }
}

fn file(status: FileStatus, path: &str, added: u64, deleted: u64) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        old_path: None,
        status,
        lines: Some(LineCounts { added, deleted }),
    }
}

#[tokio::test]
async fn lists_each_file_with_its_status_and_line_counts() {
    let ten = b"1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
    let f = Fixture::new(&[
        ("src/edit.txt", b"a\nb\n"),
        ("gone.txt", b"x\ny\nz\n"),
        ("old/name.txt", ten),
    ]);
    f.write("src/edit.txt", b"a\nB\nc\n");
    f.write("new.txt", b"hi\n");
    std::fs::remove_file(f.work.join("gone.txt")).unwrap();
    std::fs::remove_file(f.work.join("old/name.txt")).unwrap();
    f.write("moved/name.txt", b"1\n2\n3\n4\n5\n6\n7\n8\n9\nten\n");
    let head = f.open_pr().await;

    let changed = f.changed(&f.main(), &head).await.expect("lists");

    let renamed = ChangedFile {
        old_path: Some("old/name.txt".to_string()),
        ..file(FileStatus::Renamed, "moved/name.txt", 1, 1)
    };
    assert_eq!(
        changed,
        [
            file(FileStatus::Deleted, "gone.txt", 0, 3),
            renamed,
            file(FileStatus::Added, "new.txt", 1, 0),
            file(FileStatus::Modified, "src/edit.txt", 2, 1),
        ]
    );
    let stat = sh_git(&f.clone, &["diff", "--shortstat", &f.main(), &head]);
    assert_eq!(
        stat,
        "4 files changed, 4 insertions(+), 5 deletions(-)"
    );
}

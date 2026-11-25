//! `Git::changed_files` and `Git::file_diff` against temp repos: a bare repo stands in for the
//! forge, and a clone of it for the user's clone.

use std::path::{Path, PathBuf};
use std::process::Command;

use docent_git::{
    ChangedFile, FileDiff, FileStatus, Git, GitError, Hunk, LineCounts, PrWorktreePath,
    WorktreesRoot,
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
        sh_git(
            &self.work,
            &["commit", "--quiet", "--allow-empty", "-m", "x"],
        );
        sh_git(&self.work, &["rev-parse", "HEAD"])
    }

    fn dest(&self) -> PrWorktreePath {
        WorktreesRoot::new(self.root.path().join("data")).pr(1, 7)
    }

    /// Commits the scratch repo's changes as PR 7 and opens it; returns its
    /// head.
    async fn open_pr(&self) -> String {
        let head = self.commit();
        self.open_pr_at(&head).await;
        head
    }

    async fn open_pr_at(&self, head: &str) {
        let refspec = format!("{head}:refs/pull/7/head");
        sh_git(&self.work, &["push", "--quiet", "origin", &refspec]);
        git()
            .open_pr_worktree(&self.clone, "origin", &self.dest())
            .await
            .expect("opens");
    }

    /// `main` as the clone last fetched it.
    fn main(&self) -> String {
        sh_git(&self.clone, &["rev-parse", "origin/main"])
    }

    async fn listed(&self, base: &str, head: &str) -> Result<Vec<ChangedFile>, GitError> {
        git()
            .changed_files(&self.clone, "origin", &self.dest(), base, head)
            .await
    }

    /// The listing, without the blob IDs that only `file_diff` needs.
    async fn changed(&self, base: &str, head: &str) -> Result<Vec<ChangedFile>, GitError> {
        let mut files = self.listed(base, head).await?;
        for file in &mut files {
            (file.old_blob, file.new_blob) = (None, None);
        }
        Ok(files)
    }

    async fn diff(&self, file: &ChangedFile) -> Result<FileDiff, GitError> {
        let (old, new) = (file.old_blob.as_deref(), file.new_blob.as_deref());
        git().file_diff(&self.clone, &self.dest(), old, new).await
    }
}

fn file(status: FileStatus, path: &str, added: u64, deleted: u64) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        old_path: None,
        status,
        lines: Some(LineCounts { added, deleted }),
        old_blob: None,
        new_blob: None,
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
    assert_eq!(stat, "4 files changed, 4 insertions(+), 5 deletions(-)");
}

/// Commits made to the base branch after the PR branched aren't the PR's.
/// The clone doesn't have them, so the base must be fetched, without
/// touching the user's refs.
#[tokio::test]
async fn a_base_that_moved_on_is_fetched_and_its_commits_left_out() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);
    f.write("pr.txt", b"pr\n");
    let head = f.open_pr().await;
    sh_git(&f.work, &["reset", "--quiet", "--hard", "HEAD~1"]);
    f.write("base.txt", b"base\n");
    let base = f.commit();
    sh_git(&f.work, &["push", "--quiet", "origin", "main"]);
    let user_refs = ["for-each-ref", "refs/heads", "refs/remotes", "refs/tags"];
    let before = sh_git(&f.clone, &user_refs);

    let changed = f.changed(&base, &head).await.expect("lists");

    assert_eq!(changed, [file(FileStatus::Added, "pr.txt", 1, 0)]);
    assert_eq!(sh_git(&f.clone, &user_refs), before);
    git()
        .remove_pr_worktree(&f.clone, "origin", &f.dest())
        .await
        .expect("removes");
    let pins = sh_git(&f.clone, &["for-each-ref", "refs/docent"]);
    assert_eq!(pins, "", "the base's pin outlived the PR's worktree");
}

#[tokio::test]
async fn binary_files_have_no_line_counts_and_odd_names_come_back_whole() {
    let f = Fixture::new(&[(":old\tname", b"1\n2\n3\n")]);
    std::fs::rename(f.work.join(":old\tname"), f.work.join("new\n\"name\" é")).unwrap();
    f.write("blob.bin", b"\0\x01\x02");
    f.write("empty", b"");
    let head = f.open_pr().await;

    let changed = f.changed(&f.main(), &head).await.expect("lists");

    let blob = ChangedFile {
        lines: None,
        ..file(FileStatus::Added, "blob.bin", 0, 0)
    };
    let renamed = ChangedFile {
        old_path: Some(":old\tname".to_string()),
        ..file(FileStatus::Renamed, "new\n\"name\" é", 0, 0)
    };
    assert_eq!(
        changed,
        [blob, file(FileStatus::Added, "empty", 0, 0), renamed]
    );
}

/// macOS won't create such a file, so git's index does.
#[tokio::test]
async fn a_name_that_is_not_utf8_does_not_fail_the_list() {
    use std::os::unix::ffi::OsStringExt;

    let f = Fixture::new(&[("a.txt", b"a\n")]);
    let blob = sh_git(&f.work, &["rev-parse", "HEAD:a.txt"]);
    let mut entry = format!("100644,{blob},").into_bytes();
    entry.extend(b"bad\xff.txt");
    let added = Command::new("git")
        .args(["update-index", "--add", "--cacheinfo"])
        .arg(std::ffi::OsString::from_vec(entry))
        .current_dir(&f.work)
        .status();
    assert!(added.expect("run git").success());
    sh_git(&f.work, &["commit", "--quiet", "-m", "x"]);
    let head = sh_git(&f.work, &["rev-parse", "HEAD"]);
    // Never checked out: this only needs the PR's objects and a worktree.
    f.open_pr_at(&f.main()).await;
    sh_git(
        &f.work,
        &["push", "--quiet", "origin", &format!("{head}:refs/x/y")],
    );
    sh_git(&f.clone, &["fetch", "--quiet", "origin", "refs/x/y"]);

    let changed = f.changed(&f.main(), &head).await.expect("lists");

    assert_eq!(changed, [file(FileStatus::Added, "bad\u{fffd}.txt", 1, 0)]);
}

/// PR content is hostile: an attribute in it must not run a diff command,
/// nor a hook or fsmonitor, that the clone's config names.
#[tokio::test]
async fn listing_runs_none_of_the_clones_configured_commands() {
    use std::os::unix::fs::PermissionsExt;

    let f = Fixture::new(&[("a.txt", b"a\n")]);
    f.write("a.txt", b"b\n");
    f.write(".gitattributes", b"* diff=evil filter=evil\n");
    let head = f.open_pr().await;
    // A base to fetch, which could run hooks too.
    sh_git(&f.work, &["reset", "--quiet", "--hard", "HEAD~1"]);
    let base = f.commit();
    sh_git(&f.work, &["push", "--quiet", "origin", "main"]);
    let marker = f.root.path().join("PWNED");
    let script = f.clone.join(".git/evil.sh");
    let body = format!("#!/bin/sh\ntouch '{}'\n", marker.display());
    std::fs::write(&script, body).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::copy(&script, f.clone.join(".git/hooks/reference-transaction")).unwrap();
    for key in [
        "core.fsmonitor",
        "diff.external",
        "diff.evil.command",
        "diff.evil.textconv",
        "filter.evil.clean",
        "filter.evil.smudge",
    ] {
        sh_git(&f.clone, &["config", key, script.to_str().unwrap()]);
    }

    f.changed(&base, &head).await.expect("lists");

    assert!(!marker.exists(), "a clone-configured command ran");
}

/// The SHAs come through the UI, so anything else is refused before git
/// could read it as an option, a refspec or another ref.
#[tokio::test]
async fn anything_but_a_full_sha_is_refused() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);
    let head = f.open_pr().await;
    let short = &head[..12];
    let refspec = format!("{head}:refs/heads/evil");

    for bad in ["", "main", "HEAD", "--output=x", short, &refspec] {
        for (base, head) in [(bad, head.as_str()), (head.as_str(), bad)] {
            let err = f.changed(base, head).await.unwrap_err();
            assert_eq!(err, GitError::InvalidSha(bad.to_string()));
        }
    }
}

/// A worktree swapped for a symlink could lead to a repo that isn't the PR's.
#[tokio::test]
async fn a_symlinked_worktree_is_refused() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);
    let head = f.open_pr().await;
    std::fs::remove_dir_all(f.dest().path()).unwrap();
    std::os::unix::fs::symlink(&f.clone, f.dest().path()).unwrap();

    let err = f.changed(&f.main(), &head).await.unwrap_err();

    assert_eq!(err.kind(), "outside_worktrees");
}

/// A missing folder must not be reported as a missing `git`.
#[tokio::test]
async fn a_worktree_that_is_gone_says_so() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);

    let err = f.changed(&f.main(), &f.main()).await.unwrap_err();

    assert_eq!(err.kind(), "worktree_missing");
}

/// In a plain folder git would go looking for a repo above it, say one the
/// user keeps their home folder in.
#[tokio::test]
async fn a_folder_that_is_not_the_prs_worktree_is_refused() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);
    let head = f.open_pr().await;
    std::fs::remove_dir_all(f.dest().path()).unwrap();
    std::fs::create_dir(f.dest().path()).unwrap();
    sh_git(f.root.path(), &["init", "--quiet"]);

    let err = f.changed(&f.main(), &head).await.unwrap_err();

    assert_eq!(err.kind(), "worktree_missing");
}

/// Git's own `-U0` hunks for `paths` between two commits, from the test
/// machine's `git diff`.
fn git_hunks(dir: &Path, base: &str, head: &str, paths: &[&str]) -> Vec<Hunk> {
    let mut args = vec!["diff", "-U0", "-M", base, head, "--"];
    args.extend(paths);
    let count = |part: &str| -> (u64, u64) {
        let (start, lines) = part.split_once(',').unwrap_or((part, "1"));
        (start.parse().unwrap(), lines.parse().unwrap())
    };
    sh_git(dir, &args)
        .lines()
        .filter_map(|line| line.strip_prefix("@@ -"))
        .map(|line| {
            let mut parts = line.split(' ');
            let (old_start, old_lines) = count(parts.next().unwrap());
            let (new_start, new_lines) = count(&parts.next().unwrap()[1..]);
            Hunk {
                old_start,
                old_lines,
                new_start,
                new_lines,
            }
        })
        .collect()
}

fn text(diff: FileDiff) -> (String, String, Vec<Hunk>) {
    match diff {
        FileDiff::Text { old, new, hunks } => (old, new, hunks),
        other => panic!("expected a text diff, got {other:?}"),
    }
}

#[tokio::test]
async fn each_files_diff_has_its_contents_and_gits_exact_hunks() {
    // The M0 spike's cases: a pure deletion next to a blank line, and two
    // hunks one unchanged line apart.
    let spike_old = "x\nfoo\n\nfoo\nbar\n\n}\nkeep\n}\nlast\n";
    let spike_new = "x\nfoo\nbar\n\n}\nnew\n}\nLAST\n";
    let ten = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
    let f = Fixture::new(&[
        ("spike.txt", spike_old.as_bytes()),
        ("old.txt", ten.as_bytes()),
        ("gone.txt", b"a\nb\n"),
        ("tail.txt", b"a\nb"),
    ]);
    f.write("spike.txt", spike_new.as_bytes());
    std::fs::remove_file(f.work.join("old.txt")).unwrap();
    f.write("new.txt", b"1\n2\n3\n4\n5\n6\n7\n8\n9\nten\n");
    std::fs::remove_file(f.work.join("gone.txt")).unwrap();
    f.write("tail.txt", b"a\nc");
    f.write("added.txt", b"one\ntwo");
    let head = f.open_pr().await;
    let base = f.main();

    let files = f.listed(&base, &head).await.expect("lists");
    let mut diffs = Vec::new();
    for file in &files {
        diffs.push(text(f.diff(file).await.expect("diffs")));
    }

    let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(
        paths,
        ["added.txt", "gone.txt", "new.txt", "spike.txt", "tail.txt"]
    );
    let wanted: [(&str, &str, &[&str]); 5] = [
        ("", "one\ntwo", &["added.txt"]),
        ("a\nb\n", "", &["gone.txt"]),
        (
            ten,
            "1\n2\n3\n4\n5\n6\n7\n8\n9\nten\n",
            &["old.txt", "new.txt"],
        ),
        (spike_old, spike_new, &["spike.txt"]),
        ("a\nb", "a\nc", &["tail.txt"]),
    ];
    for ((old, new, hunks), (want_old, want_new, paths)) in diffs.into_iter().zip(wanted) {
        assert_eq!((old.as_str(), new.as_str()), (want_old, want_new));
        assert_eq!(hunks, git_hunks(&f.clone, &base, &head, paths), "{paths:?}");
    }
}

#[tokio::test]
async fn the_listing_names_each_sides_blob() {
    let f = Fixture::new(&[("a.txt", b"a\n"), ("gone.txt", b"g\n")]);
    f.write("a.txt", b"b\n");
    std::fs::remove_file(f.work.join("gone.txt")).unwrap();
    let head = f.open_pr().await;
    let base = f.main();

    let files = f.listed(&base, &head).await.expect("lists");

    let blob = |commit: &str, path: &str| {
        Some(sh_git(
            &f.clone,
            &["rev-parse", &format!("{commit}:{path}")],
        ))
    };
    let blobs: Vec<_> = files
        .iter()
        .map(|f| (f.old_blob.clone(), f.new_blob.clone()))
        .collect();
    assert_eq!(
        blobs,
        [
            (blob(&base, "a.txt"), blob(&head, "a.txt")),
            (blob(&base, "gone.txt"), None),
        ]
    );
}

#[tokio::test]
async fn binary_and_very_large_files_are_not_read_as_text() {
    let f = Fixture::new(&[("blob.bin", b"a\0b"), ("big.txt", b"small\n")]);
    f.write("blob.bin", b"a\0c");
    let big = "line\n".repeat(1024 * 1024);
    f.write("big.txt", big.as_bytes());
    let head = f.open_pr().await;

    let files = f.listed(&f.main(), &head).await.expect("lists");

    assert_eq!(
        f.diff(&files[0]).await.expect("diffs"),
        FileDiff::TooLarge {
            bytes: big.len() as u64
        }
    );
    assert_eq!(f.diff(&files[1]).await.expect("diffs"), FileDiff::Binary);
}

/// Two names that aren't UTF-8 can look the same once their bad bytes are
/// replaced; each must still show its own diff.
#[tokio::test]
async fn look_alike_names_that_are_not_utf8_each_show_their_own_diff() {
    use std::os::unix::ffi::OsStringExt;

    let f = Fixture::new(&[("a.txt", b"a\n")]);
    for (name, contents) in [(&b"bad\xfe"[..], "first\n"), (b"bad\xff", "second\n")] {
        f.write("tmp", contents.as_bytes());
        let blob = sh_git(&f.work, &["hash-object", "-w", "tmp"]);
        let mut entry = format!("100644,{blob},").into_bytes();
        entry.extend(name);
        let added = Command::new("git")
            .args(["update-index", "--add", "--cacheinfo"])
            .arg(std::ffi::OsString::from_vec(entry))
            .current_dir(&f.work)
            .status();
        assert!(added.expect("run git").success());
    }
    std::fs::remove_file(f.work.join("tmp")).unwrap();
    sh_git(&f.work, &["commit", "--quiet", "-m", "x"]);
    let head = sh_git(&f.work, &["rev-parse", "HEAD"]);
    f.open_pr_at(&f.main()).await;
    sh_git(
        &f.work,
        &["push", "--quiet", "origin", &format!("{head}:refs/x/y")],
    );
    sh_git(&f.clone, &["fetch", "--quiet", "origin", "refs/x/y"]);

    let files = f.listed(&f.main(), &head).await.expect("lists");
    let mut news = Vec::new();
    for file in &files {
        news.push(text(f.diff(file).await.expect("diffs")).1);
    }

    assert_eq!(files[0].path, files[1].path);
    assert_eq!(news, ["first\n", "second\n"]);
}

/// The blob IDs come through the UI, so anything else is refused before git
/// could read it as an option or a ref.
#[tokio::test]
async fn a_diff_of_anything_but_full_shas_is_refused() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);
    let head = f.open_pr().await;
    let blob = sh_git(&f.clone, &["rev-parse", &format!("{head}:a.txt")]);

    for bad in ["", "HEAD:a.txt", "--output=x", &blob[..12]] {
        for (old, new) in [(Some(bad), Some(blob.as_str())), (None, Some(bad))] {
            let err = git()
                .file_diff(&f.clone, &f.dest(), old, new)
                .await
                .unwrap_err();
            assert_eq!(err, GitError::InvalidSha(bad.to_string()));
        }
    }
}

/// PR content is hostile: diffing must run none of the commands the clone's
/// config names.
#[tokio::test]
async fn a_diff_runs_none_of_the_clones_configured_commands() {
    use std::os::unix::fs::PermissionsExt;

    let f = Fixture::new(&[("a.txt", b"a\n")]);
    f.write("a.txt", b"b\n");
    f.write(".gitattributes", b"* diff=evil\n");
    let head = f.open_pr().await;
    let marker = f.root.path().join("PWNED");
    let script = f.clone.join(".git/evil.sh");
    let body = format!("#!/bin/sh\ntouch '{}'\n", marker.display());
    std::fs::write(&script, body).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    for key in [
        "core.fsmonitor",
        "diff.external",
        "diff.evil.command",
        "diff.evil.textconv",
    ] {
        sh_git(&f.clone, &["config", key, script.to_str().unwrap()]);
    }
    let files = f.listed(&f.main(), &head).await.expect("lists");

    for file in &files {
        f.diff(file).await.expect("diffs");
    }

    assert!(!marker.exists(), "a clone-configured command ran");
}

#[tokio::test]
async fn a_diff_in_a_worktree_that_is_gone_says_so() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);
    let blob = sh_git(&f.clone, &["rev-parse", "HEAD:a.txt"]);

    let err = git()
        .file_diff(&f.clone, &f.dest(), None, Some(&blob))
        .await
        .unwrap_err();

    assert_eq!(err.kind(), "worktree_missing");
}

/// A submodule's side names a commit in another repo, which isn't a blob
/// this clone can read.
#[tokio::test]
async fn a_submodule_has_no_blob_to_diff() {
    let f = Fixture::new(&[("a.txt", b"a\n")]);
    let elsewhere = "1".repeat(40);
    let entry = format!("160000,{elsewhere},sub");
    sh_git(&f.work, &["update-index", "--add", "--cacheinfo", &entry]);
    // Straight from the index: there's no folder for `add` to find.
    sh_git(&f.work, &["commit", "--quiet", "-m", "x"]);
    let head = sh_git(&f.work, &["rev-parse", "HEAD"]);
    f.open_pr_at(&head).await;

    let files = f.listed(&f.main(), &head).await.expect("lists");

    assert_eq!(files.len(), 1);
    assert_eq!((&files[0].old_blob, &files[0].new_blob), (&None, &None));
}

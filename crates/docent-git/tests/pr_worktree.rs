//! `Git::open_pr_worktree` and `Git::remove_pr_worktree` against temp
//! repos: a bare repo stands in for the forge, and a clone of it for the
//! user's clone.

use std::path::{Path, PathBuf};
use std::process::Command;

use docent_git::{Git, PrWorktreePath, WorktreesRoot};

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

    /// Where PR `number` of repo 1 goes.
    fn dest(&self, number: u64) -> PrWorktreePath {
        WorktreesRoot::new(&self.data).pr(1, number)
    }

    fn worktree(&self, number: u64) -> PathBuf {
        self.dest(number).path().to_path_buf()
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
        .open_pr_worktree(&f.clone, "origin", &f.dest(7))
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
        .open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens");
    // Rewrite history: the new head doesn't contain the old one.
    sh_git(&f.work, &["reset", "--quiet", "--hard", "HEAD~1"]);
    let rewritten = f.commit("feature.txt", "rewritten\n");
    f.publish_pr(7, &rewritten);
    let before = f.user_state();

    let reopened = git()
        .open_pr_worktree(&f.clone, "origin", &f.dest(7))
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
    let dest = f.dest(7);
    git.open_pr_worktree(&f.clone, "origin", &dest)
        .await
        .expect("opens");

    let reopened = git
        .open_pr_worktree(&f.clone, "origin", &dest)
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
    let dest = f.dest(7);
    git.open_pr_worktree(&f.clone, "origin", &dest)
        .await
        .expect("opens");
    std::fs::remove_dir_all(dest.path()).expect("delete the worktree folder");

    let reopened = git
        .open_pr_worktree(&f.clone, "origin", &dest)
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
    git.open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens");
    // The re-open path checks out inside the worktree.
    let second = f.commit("more.txt", "more\n");
    f.publish_pr(7, &second);

    git.open_pr_worktree(&f.clone, "origin", &f.dest(7))
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
        .open_pr_worktree(&f.clone, "origin", &f.dest(7))
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
        .open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens");

    assert_eq!(sh_git(&opened.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(f.user_state(), before, "the user's branch or edits changed");
    assert_eq!(
        std::fs::read_to_string(f.clone.join("feature.txt")).unwrap(),
        "my edit\n"
    );
}

/// The `refs/docent` refs in `clone`, one per line.
fn docent_refs(clone: &Path) -> String {
    sh_git(
        clone,
        &["for-each-ref", "--format=%(refname)", "refs/docent"],
    )
}

/// PR 7's removal must leave PR 70 alone: `pull/7` is a prefix of `pull/70`.
#[tokio::test]
async fn removing_a_pr_worktree_deletes_its_folder_registration_and_pins() {
    let f = Fixture::new();
    let git = git();
    let seven = f.commit("seven.txt", "7\n");
    f.publish_pr(7, &seven);
    let seventy = f.commit("seventy.txt", "70\n");
    f.publish_pr(70, &seventy);
    git.open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens 7");
    git.open_pr_worktree(&f.clone, "origin", &f.dest(70))
        .await
        .expect("opens 70");
    let before = f.user_state();

    // Removing twice is a no-op the second time.
    for _ in 0..2 {
        git.remove_pr_worktree(&f.clone, "origin", &f.dest(7))
            .await
            .expect("removes");
    }

    assert!(!f.worktree(7).exists(), "the folder is still there");
    let listed = sh_git(&f.clone, &["worktree", "list", "--porcelain"]);
    assert!(!listed.contains("pr-7\n"), "still registered: {listed}");
    assert!(
        listed.contains("pr-70\n"),
        "PR 70's worktree went: {listed}"
    );
    assert_eq!(
        docent_refs(&f.clone),
        format!("refs/docent/origin/pull/70/{seventy}\nrefs/docent/origin/pull/70/head")
    );
    assert_eq!(f.user_state(), before, "the user's clone changed");
}

/// The user may have deleted the folder; git still lists it until the
/// registration goes. Removing again afterwards is a no-op.
#[tokio::test]
async fn removing_a_worktree_whose_folder_is_gone_drops_its_registration() {
    let f = Fixture::new();
    let git = git();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    git.open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens");
    std::fs::remove_dir_all(f.worktree(7)).expect("delete the folder");

    for _ in 0..2 {
        git.remove_pr_worktree(&f.clone, "origin", &f.dest(7))
            .await
            .expect("removes");
    }

    let listed = sh_git(&f.clone, &["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
    assert_eq!(docent_refs(&f.clone), "");
}

/// Paths whose folders on disk are symlinks to `mine`: repo 1's PR 7 is one,
/// and repo 8's whole folder leads to the parent of `mine`, named `pr-7`.
#[cfg(unix)]
fn symlinked_dests(f: &Fixture, mine: &Path) -> Vec<PrWorktreePath> {
    use std::os::unix::fs::symlink;

    std::fs::create_dir_all(f.data.join("1")).expect("repo folder");
    symlink(mine, f.worktree(7)).expect("symlink");
    symlink(mine.parent().unwrap(), f.data.join("8")).expect("symlink");
    vec![f.dest(7), WorktreesRoot::new(&f.data).pr(8, 7)]
}

/// A worktree's path comes from its repo id and PR number alone.
#[test]
fn worktree_paths_are_derived_inside_the_root() {
    let dest = WorktreesRoot::new(Path::new("/data/worktrees")).pr(3, 7);

    assert_eq!(dest.path(), Path::new("/data/worktrees/3/pr-7"));
}

/// Removal deletes folders, so it only ever acts inside Docent's worktrees
/// folder: never the user's own worktree, however the path reaches it.
#[cfg(unix)]
#[tokio::test]
async fn removing_refuses_any_path_outside_the_worktrees_folder() {
    let f = Fixture::new();
    let git = git();
    let mine = f.data.parent().unwrap().join("pr-7");
    sh_git(
        &f.clone,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "mine",
            mine.to_str().unwrap(),
        ],
    );
    std::fs::write(mine.join("work.txt"), "unsaved\n").expect("edit");
    let dests = symlinked_dests(&f, &mine);
    let before = f.user_state();
    let listed_before = sh_git(&f.clone, &["worktree", "list", "--porcelain"]);

    for dest in dests {
        let err = git
            .remove_pr_worktree(&f.clone, "origin", &dest)
            .await
            .expect_err(&format!("removed {}", dest.path().display()));
        assert_eq!(err.kind(), "outside_worktrees", "{dest:?}: {err}");
    }

    assert_eq!(
        std::fs::read_to_string(mine.join("work.txt")).unwrap(),
        "unsaved\n"
    );
    assert_eq!(
        sh_git(&f.clone, &["worktree", "list", "--porcelain"]),
        listed_before
    );
    assert!(f.worktree(7).symlink_metadata().is_ok());
    assert_eq!(f.user_state(), before, "the user's clone changed");
}

/// A crash mid-create can leave a folder that git doesn't list. It must not
/// block opening the PR again.
#[tokio::test]
async fn a_stray_folder_at_the_worktree_path_is_cleared_on_open() {
    let f = Fixture::new();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    std::fs::create_dir_all(f.worktree(7)).expect("stray folder");
    std::fs::write(f.worktree(7).join("half-written"), "x").expect("write");

    let opened = git()
        .open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens");

    assert_eq!(sh_git(&opened.path, &["rev-parse", "HEAD"]), head);
    assert!(!opened.path.join("half-written").exists());
}

#[tokio::test]
async fn a_stray_folder_at_the_worktree_path_is_cleared_on_remove() {
    let f = Fixture::new();
    std::fs::create_dir_all(f.worktree(7)).expect("stray folder");
    std::fs::write(f.worktree(7).join("half-written"), "x").expect("write");

    git()
        .remove_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("removes");

    assert!(!f.worktree(7).exists());
}

/// Opening checks out and clears folders, so it is held to the same
/// boundary as removal, before it fetches anything: never a folder outside
/// Docent's worktrees folder, nor the user's own worktree.
#[cfg(unix)]
#[tokio::test]
async fn opening_refuses_any_path_outside_the_worktrees_folder() {
    let f = Fixture::new();
    let git = git();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    let base = f.data.parent().unwrap().to_path_buf();
    let mine = base.join("pr-7");
    sh_git(
        &f.clone,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "mine",
            mine.to_str().unwrap(),
        ],
    );
    std::fs::write(mine.join("work.txt"), "unsaved\n").expect("edit");
    let dests = symlinked_dests(&f, &mine);
    let before = f.user_state();
    let listed_before = sh_git(&f.clone, &["worktree", "list", "--porcelain"]);

    for dest in dests {
        let err = git
            .open_pr_worktree(&f.clone, "origin", &dest)
            .await
            .expect_err(&format!("opened at {}", dest.path().display()));
        assert_eq!(err.kind(), "outside_worktrees", "{dest:?}: {err}");
    }

    assert_eq!(
        std::fs::read_to_string(mine.join("work.txt")).unwrap(),
        "unsaved\n"
    );
    assert_eq!(
        sh_git(&f.clone, &["worktree", "list", "--porcelain"]),
        listed_before
    );
    assert_eq!(docent_refs(&f.clone), "", "a refused open fetched");
    assert_eq!(f.user_state(), before, "the user's clone changed");
}

/// git's ref patterns are globs. A remote whose name has glob characters
/// (possible only by editing config by hand) must not match another
/// remote's pins.
#[tokio::test]
async fn removing_deletes_only_its_own_remotes_pins() {
    let f = Fixture::new();
    let git = git();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    git.open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens");
    let pins = docent_refs(&f.clone);

    git.remove_pr_worktree(&f.clone, "o*", &WorktreesRoot::new(&f.data).pr(2, 7))
        .await
        .expect("removes");

    assert_eq!(docent_refs(&f.clone), pins);
}

/// Deleting refs fires `reference-transaction`, and removing a worktree
/// reads the clone's config: none of the clone's commands may run.
#[cfg(unix)]
#[tokio::test]
async fn removing_a_pr_runs_none_of_the_clones_configured_commands() {
    use std::os::unix::fs::PermissionsExt;

    let f = Fixture::new();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    let git = git();
    git.open_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("opens");
    let marker = f.data.parent().unwrap().join("PWNED");
    let script = f.clone.join(".git/evil.sh");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .expect("write script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    for hook in ["reference-transaction", "post-checkout", "pre-auto-gc"] {
        std::fs::copy(&script, f.clone.join(".git/hooks").join(hook)).expect("install hook");
    }
    sh_git(
        &f.clone,
        &["config", "core.fsmonitor", script.to_str().unwrap()],
    );
    sh_git(&f.clone, &["config", "gc.auto", "1"]);

    git.remove_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect("removes");

    assert_eq!(docent_refs(&f.clone), "", "the pins weren't removed");
    assert!(!marker.exists(), "a clone-configured command ran");
}

/// A stray folder Docent can't delete is reported as such, not as a git
/// failure: git never ran.
#[cfg(unix)]
#[tokio::test]
async fn a_stray_folder_that_cannot_be_cleared_says_so() {
    use std::os::unix::fs::PermissionsExt;

    let f = Fixture::new();
    let locked = f.worktree(7).join("locked");
    std::fs::create_dir_all(&locked).expect("stray folder");
    std::fs::write(locked.join("file"), "x").expect("write");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).expect("chmod");

    let err = git()
        .remove_pr_worktree(&f.clone, "origin", &f.dest(7))
        .await
        .expect_err("cleared a read-only folder");

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    assert_eq!(err.kind(), "clear_failed", "{err}");
}

/// git's arguments here must be UTF-8.
#[cfg(unix)]
#[tokio::test]
async fn a_non_utf8_worktree_path_is_reported_as_invalid() {
    use std::os::unix::ffi::OsStrExt;

    let f = Fixture::new();
    let root = f.data.join(std::ffi::OsStr::from_bytes(b"data-\xff"));
    let dest = WorktreesRoot::new(root).pr(1, 7);

    let err = git()
        .open_pr_worktree(&f.clone, "origin", &dest)
        .await
        .expect_err("opened at a non-UTF-8 path");

    assert_eq!(err.kind(), "invalid_path", "{err}");
    assert_eq!(docent_refs(&f.clone), "", "fetched before refusing");
}

/// Staying inside the root isn't enough: a repo's folder that is a symlink to
/// another repo's must not let one reach the other's worktrees.
#[cfg(unix)]
#[tokio::test]
async fn a_repo_folder_linked_to_another_repos_is_refused() {
    let f = Fixture::new();
    let head = f.commit("feature.txt", "new\n");
    f.publish_pr(7, &head);
    let git = git();
    std::fs::create_dir_all(f.worktree(7)).expect("repo 1's folder");
    std::fs::write(f.worktree(7).join("keep.txt"), "x").expect("write");
    std::os::unix::fs::symlink(f.data.join("1"), f.data.join("8")).expect("symlink");
    let dest = WorktreesRoot::new(&f.data).pr(8, 7);

    let removed = git.remove_pr_worktree(&f.clone, "origin", &dest).await;
    let opened = git.open_pr_worktree(&f.clone, "origin", &dest).await;

    assert_eq!(removed.unwrap_err().kind(), "outside_worktrees");
    assert_eq!(opened.unwrap_err().kind(), "outside_worktrees");
    assert!(f.worktree(7).join("keep.txt").exists());
}

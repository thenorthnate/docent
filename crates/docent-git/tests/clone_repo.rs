//! `Git::clone_repo` against a local bare repo, which stands in for the forge:
//! the user's config rewrites the forge's URL to ssh, and ssh is a script that
//! serves the bare repo.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use docent_git::{BoundClone, CloneProgress, Git, GitError};

fn sh_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(args)
        .current_dir(dir)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

struct Fixture {
    _root: tempfile::TempDir,
    /// The user's home, holding their `.gitconfig`.
    home: PathBuf,
    /// The folder the user picks to clone into.
    parent: PathBuf,
}

impl Fixture {
    /// A forge whose repo has `files` on `main`.
    fn new(files: &[(&str, &str)]) -> Self {
        let root = tempfile::tempdir().expect("temp dir");
        let base = root.path().canonicalize().expect("canonicalize");
        let (work, home, parent) = (base.join("work"), base.join("home"), base.join("code"));
        for dir in [&work, &home, &parent] {
            std::fs::create_dir(dir).expect("mkdir");
        }
        sh_git(&work, &["init", "--quiet", "-b", "main"]);
        for (name, contents) in files {
            let path = work.join(name);
            std::fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
            std::fs::write(&path, contents).expect("write");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        sh_git(&work, &["add", "."]);
        sh_git(&work, &["commit", "--quiet", "--allow-empty", "-m", "x"]);
        sh_git(&base, &["clone", "--quiet", "--bare", "work", "forge.git"]);
        let fixture = Self {
            _root: root,
            home,
            parent,
        };
        let forge = base.join("forge.git");
        fixture.ssh(&format!("exec git upload-pack '{}'", forge.display()));
        fixture
    }

    /// Makes `script` the user's ssh, which every clone goes through.
    fn ssh(&self, script: &str) {
        let ssh = self.home.join("ssh.sh");
        std::fs::write(&ssh, format!("{script}\n")).expect("write ssh");
        self.config(&format!(
            "[url \"git@github.com:\"]\n\tinsteadOf = https://github.com/\n\
             [core]\n\tsshCommand = sh '{}'\n\
             [ssh]\n\tvariant = simple\n",
            ssh.display()
        ));
    }

    fn config(&self, text: &str) {
        std::fs::write(self.home.join(".gitconfig"), text).expect("write config");
    }

    fn git(&self) -> Git {
        let path = std::env::var_os("PATH").expect("tests run with a PATH");
        Git::with_env(path, [("HOME".into(), self.home.clone().into())])
    }

    async fn clone_named(&self, owner: &str, name: &str) -> Result<BoundClone, GitError> {
        self.git()
            .clone_repo(&self.parent, "github.com", owner, name, |_| {})
            .await
    }

    async fn clone(&self) -> Result<BoundClone, GitError> {
        self.clone_named("octocat", "Hello-World").await
    }

    /// What the parent folder holds.
    fn cloned(&self) -> Vec<String> {
        std::fs::read_dir(&self.parent)
            .expect("read parent")
            .map(|e| e.expect("entry").file_name().into_string().expect("utf-8"))
            .collect()
    }
}

#[tokio::test]
async fn a_clone_is_full_and_named_after_the_repo() {
    let fixture = Fixture::new(&[("README", "hello\n")]);

    let clone = fixture.clone().await.expect("clones");

    assert_eq!(clone.path, fixture.parent.join("Hello-World"));
    assert_eq!(clone.remote, "origin");
    let readme = std::fs::read_to_string(clone.path.join("README")).expect("checked out");
    assert_eq!(readme, "hello\n");
    assert!(!clone.path.join(".git/shallow").exists());
}

#[tokio::test]
async fn progress_is_reported_while_cloning() {
    let fixture = Fixture::new(&[("README", "hello\n")]);
    let mut seen: Vec<CloneProgress> = Vec::new();

    fixture
        .git()
        .clone_repo(
            &fixture.parent,
            "github.com",
            "octocat",
            "Hello-World",
            |p| seen.push(p),
        )
        .await
        .expect("clones");

    assert!(
        seen.contains(&CloneProgress {
            phase: "Receiving objects".to_string(),
            percent: 100
        }),
        "{seen:?}"
    );
}

#[tokio::test]
async fn a_failed_clone_shows_gits_error_and_leaves_no_folder() {
    let fixture = Fixture::new(&[]);
    fixture.ssh("echo 'ssh: Could not resolve hostname github.com' >&2; exit 255");

    let err = fixture.clone().await.unwrap_err();

    assert_eq!(err.kind(), "git_failed");
    assert!(
        err.to_string().contains("Could not resolve hostname"),
        "{err}"
    );
    assert!(fixture.cloned().is_empty(), "{:?}", fixture.cloned());
}

/// A server that sends nothing is given up on, and its half-made clone goes.
#[tokio::test]
async fn a_clone_that_stalls_is_stopped_and_leaves_no_folder() {
    let fixture = Fixture::new(&[]);
    let pid_file = fixture.home.join("ssh.pid");
    fixture.ssh(&format!(
        "echo $$ > '{}'\nexec sleep 30",
        pid_file.display()
    ));

    let started = std::time::Instant::now();
    let err = fixture
        .git()
        .with_stall_limit(Duration::from_secs(2))
        .clone_repo(
            &fixture.parent,
            "github.com",
            "octocat",
            "Hello-World",
            |_| {},
        )
        .await
        .unwrap_err();

    assert!(err.to_string().contains("stopped responding"), "{err}");
    // ssh is stopped along with git, so it can't keep writing.
    let pid = std::fs::read_to_string(&pid_file).expect("ssh ran");
    let gone = (0..50).any(|_| {
        std::thread::sleep(Duration::from_millis(50));
        let alive = Command::new("kill").args(["-0", pid.trim()]).output();
        !alive.expect("run kill").status.success()
    });
    assert!(gone, "ssh is still running");
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(fixture.cloned().is_empty(), "{:?}", fixture.cloned());
}

/// ssh connection sharing leaves a process behind that holds git's stderr.
#[tokio::test]
async fn a_process_left_behind_by_git_does_not_hold_up_the_clone() {
    let fixture = Fixture::new(&[("README", "hello\n")]);
    let ssh = std::fs::read_to_string(fixture.home.join("ssh.sh")).expect("read ssh");
    fixture.ssh(&format!("sleep 30 </dev/null >/dev/null &\n{ssh}"));

    let started = std::time::Instant::now();
    fixture.clone().await.expect("clones");

    assert!(started.elapsed() < Duration::from_secs(20));
}

/// Answers the check of where the clone's URL leads, in a fake git.
const LS_REMOTE: &str =
    "case \"$*\" in ls-remote*) echo https://github.com/octocat/Hello-World.git; exit 0;; esac";

/// The forge's own messages are text it chose.
#[tokio::test]
async fn the_forge_cannot_fake_a_credentials_error() {
    let fixture = Fixture::new(&[]);
    fixture.ssh("echo 'remote: Authentication failed for x' >&2; exit 255");

    assert_eq!(fixture.clone().await.unwrap_err().kind(), "git_failed");
}

/// A clone whose remote isn't the forge can't be bound, so it isn't started.
#[tokio::test]
async fn a_url_redirected_to_another_host_is_refused_before_cloning() {
    let fixture = Fixture::new(&[]);
    let asked = fixture.home.join("asked");
    fixture.ssh(&format!("touch '{}'", asked.display()));
    let config = std::fs::read_to_string(fixture.home.join(".gitconfig")).expect("read");
    fixture.config(&config.replace("git@github.com:", "git@work-alias:"));

    let err = fixture.clone().await.unwrap_err();

    assert_eq!(err.kind(), "redirected");
    assert!(err.to_string().contains("git@work-alias:octocat"), "{err}");
    assert!(!asked.exists(), "the clone was started");
    assert!(fixture.cloned().is_empty(), "{:?}", fixture.cloned());
}

/// Only a fake git can fail this way without the network.
#[tokio::test]
async fn missing_credentials_say_how_to_set_them_up() {
    for stderr in [
        "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
        "fatal: Authentication failed for 'https://github.com/octocat/Hello-World.git/'",
    ] {
        let bin = tempfile::tempdir().expect("temp dir");
        let parent = tempfile::tempdir().expect("temp dir");
        let fake = bin.path().join("git");
        std::fs::write(
            &fake,
            format!("#!/bin/sh\n{LS_REMOTE}\necho \"{stderr}\" >&2\nexit 128\n"),
        )
        .expect("write fake git");
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let err = Git::new(bin.path().as_os_str().to_owned())
            .clone_repo(
                parent.path(),
                "github.com",
                "octocat",
                "Hello-World",
                |_| {},
            )
            .await
            .unwrap_err();

        assert_eq!(err.kind(), "no_credentials", "{err:?}");
        assert!(
            err.to_string()
                .contains("gh auth setup-git --hostname github.com"),
            "{err}"
        );
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
    }
}

/// Cleaning up after a failure must never reach what the user already had.
#[tokio::test]
async fn an_existing_folder_is_refused_and_left_alone() {
    let fixture = Fixture::new(&[]);
    let existing = fixture.parent.join("Hello-World");
    std::fs::create_dir(&existing).expect("mkdir");

    let empty = fixture.clone().await.unwrap_err();
    assert_eq!(empty.kind(), "folder_exists");
    assert!(existing.is_dir());

    std::fs::write(existing.join("mine.txt"), "keep").expect("write");
    let full = fixture.clone().await.unwrap_err();
    assert_eq!(full.kind(), "folder_exists");
    assert!(full.to_string().contains("Bind"), "{full}");
    assert!(existing.join("mine.txt").exists());
}

/// A dangling symlink doesn't "exist", but cloning through it would write
/// wherever it points.
#[tokio::test]
async fn a_symlink_where_the_clone_would_go_is_refused() {
    let fixture = Fixture::new(&[]);
    let elsewhere = fixture.home.join("elsewhere");
    std::os::unix::fs::symlink(&elsewhere, fixture.parent.join("Hello-World")).expect("symlink");

    let err = fixture.clone().await.unwrap_err();

    assert_eq!(err.kind(), "folder_exists");
    assert!(!elsewhere.exists());
}

#[tokio::test]
async fn a_missing_parent_folder_is_an_error() {
    let fixture = Fixture::new(&[]);
    std::fs::remove_dir(&fixture.parent).expect("rmdir");

    let err = fixture.clone().await.unwrap_err();

    assert_eq!(err.kind(), "create_failed");
}

/// The forge supplies the name, so it can't be allowed to pick the folder.
#[tokio::test]
async fn a_name_that_is_not_a_plain_name_is_refused() {
    let fixture = Fixture::new(&[]);
    for name in [
        "", ".", "..", ".git", ".GIT", "../up", "a/b", "/tmp/x", "-x", "a b", "a\n", "a?b", "é",
    ] {
        for (owner, name) in [("octocat", name), (name, "Hello-World")] {
            let err = fixture.clone_named(owner, name).await.unwrap_err();
            assert_eq!(err.kind(), "invalid_name", "{owner:?} / {name:?}");
        }
    }
    for host in ["", "-x", "github.com/evil", "me@github.com"] {
        let err = fixture
            .git()
            .clone_repo(&fixture.parent, host, "octocat", "Hello-World", |_| {})
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "invalid_name", "{host:?}");
    }
    assert!(fixture.cloned().is_empty(), "{:?}", fixture.cloned());
}

#[tokio::test]
async fn a_name_starting_with_a_dot_or_holding_one_is_fine() {
    let fixture = Fixture::new(&[]);

    for name in [".github", "next.js", "a_b-c"] {
        fixture.clone_named("octocat", name).await.expect("clones");
    }
}

/// A relative `core.hooksPath` in the user's config would run hooks from the
/// repo being cloned.
#[tokio::test]
async fn cloning_runs_none_of_the_repos_files_as_hooks() {
    let fixture = Fixture::new(&[(".githooks/post-checkout", "#!/bin/sh\ntouch PWNED\n")]);
    let config = std::fs::read_to_string(fixture.home.join(".gitconfig")).expect("read");
    fixture.config(&format!("{config}[core]\n\thooksPath = .githooks\n"));

    let clone = fixture.clone().await.expect("clones");

    assert!(clone.path.join(".githooks/post-checkout").exists());
    assert!(!clone.path.join("PWNED").exists(), "a hook ran");
}

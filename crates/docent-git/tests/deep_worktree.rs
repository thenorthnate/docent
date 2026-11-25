//! A hostile PR can commit folders nested deeper than the number of files
//! Docent may open. On its own here because it lowers that limit for the
//! whole process.
#![cfg(unix)]

use std::path::Path;

use docent_git::{Git, WorktreesRoot};
use rustix::process::{Resource, getrlimit, setrlimit};

#[tokio::test]
async fn a_tree_deeper_than_the_open_file_limit_is_measured_and_cleared() {
    let data = tempfile::tempdir().expect("temp dir");
    let dest = WorktreesRoot::new(data.path()).pr(1, 7);
    let deepest = dest.path().join("d/".repeat(300));
    std::fs::create_dir_all(&deepest).expect("deep tree");
    std::fs::write(deepest.join("file"), vec![1; 1_000_000]).expect("write");
    let mut limit = getrlimit(Resource::Nofile);
    limit.current = Some(128);
    setrlimit(Resource::Nofile, limit).expect("lower the limit");

    let used = dest.disk_use().expect("measures");
    Git::new("/usr/bin".into())
        .remove_pr_worktree(Path::new("/no/such/clone"), "origin", &dest)
        .await
        .expect("clears");

    assert!(used >= 1_000_000, "{used}");
    assert!(!dest.path().exists());
}

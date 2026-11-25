//! The files a PR changes, for the Review drawer (PLAN §5.4).

use std::path::Path;

use docent_git::{ChangedFile, Git, WorktreesRoot};
use docent_store::Store;

use crate::command_error::CommandError;

/// The PR whose files are wanted, and the commits to compare: the forge's
/// base for it, and the head its worktree was opened at.
#[derive(Debug, serde::Deserialize)]
pub struct PullRange {
    pub(crate) host: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) number: u64,
    pub(crate) base_sha: String,
    pub(crate) head_sha: String,
}

/// The files `range`'s PR changes, read from its open worktree. Watched and
/// bound repos only; the bound clone is checked again first, as on opening.
pub async fn changed_files(
    store: &Store,
    git: &Git,
    worktrees: &Path,
    range: &PullRange,
) -> Result<Vec<ChangedFile>, CommandError> {
    let repo = store
        .watched_repo_named(&range.host, &range.owner, &range.repo)
        .await?;
    let Some(local_path) = &repo.local_path else {
        return Err(CommandError {
            kind: "unbound",
            message: format!("{}/{} isn't bound to a local clone.", repo.owner, repo.name),
        });
    };
    let clone = git
        .check_clone(Path::new(local_path), &repo.host, &repo.owner, &repo.name)
        .await?;
    let dest = WorktreesRoot::new(worktrees).pr(repo.id, range.number);
    let (base, head) = (&range.base_sha, &range.head_sha);
    Ok(git
        .changed_files(&clone.path, &clone.remote, &dest, base, head)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree;
    use docent_store::NewRepo;

    const SHA: &str = "6dcb09b5b57875f334f61aebed695e2e4193db5e";

    fn range(base_sha: &str, head_sha: &str) -> PullRange {
        PullRange {
            host: "github.com".into(),
            owner: "octocat".into(),
            repo: "Hello-World".into(),
            number: 7,
            base_sha: base_sha.into(),
            head_sha: head_sha.into(),
        }
    }

    fn git() -> Git {
        Git::new(std::env::var_os("PATH").expect("tests run with a PATH"))
    }

    async fn watch_hello_world(store: &Store) -> docent_store::WatchedRepo {
        let account = store.upsert_account("github.com", "octocat").await.unwrap();
        let repo = NewRepo {
            owner: "octocat".into(),
            name: "Hello-World".into(),
            default_branch: "main".into(),
        };
        store.watch_repo(account, &repo).await.unwrap()
    }

    #[tokio::test]
    async fn an_opened_prs_files_reach_the_ui() {
        let store = Store::open_in_memory().await.unwrap();
        let repo = watch_hello_world(&store).await;
        let (folder, data) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let clone = folder.path().canonicalize().unwrap();
        let sh_git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .current_dir(&clone)
                .output()
                .expect("run git");
            assert!(output.status.success(), "git {args:?}");
            String::from_utf8(output.stdout).unwrap().trim().to_string()
        };
        // The clone is its own forge, reached through a stand-in for ssh.
        let path = clone.to_str().unwrap();
        let ssh = format!("sh -c 'exec git upload-pack {path}' ssh");
        sh_git(&["init", "--quiet", "-b", "main"]);
        sh_git(&["commit", "--quiet", "--allow-empty", "-m", "base"]);
        let base = sh_git(&["rev-parse", "HEAD"]);
        let url = "git@github.com:octocat/Hello-World.git";
        sh_git(&["remote", "add", "up", url]);
        sh_git(&["config", "core.sshCommand", &ssh]);
        sh_git(&["config", "ssh.variant", "simple"]);
        std::fs::write(clone.join("new.txt"), "hi\n").unwrap();
        sh_git(&["add", "."]);
        sh_git(&["commit", "--quiet", "-m", "pr"]);
        let head = sh_git(&["rev-parse", "HEAD"]);
        sh_git(&["update-ref", "refs/pull/7/head", "main"]);
        store.bind_repo(repo.id, path).await.unwrap();
        let (git, root) = (git(), data.path());
        worktree::open(
            &store,
            &git,
            root,
            "github.com",
            "octocat",
            "Hello-World",
            7,
        )
        .await
        .expect("opens");

        let files = changed_files(&store, &git, root, &range(&base, &head))
            .await
            .expect("lists");

        assert_eq!(
            serde_json::to_value(files).unwrap(),
            serde_json::json!([{
                "path": "new.txt",
                "old_path": null,
                "status": "added",
                "lines": { "added": 1, "deleted": 0 },
            }])
        );
    }

    #[tokio::test]
    async fn an_unwatched_or_unbound_repo_is_refused() {
        let store = Store::open_in_memory().await.unwrap();
        let data = tempfile::tempdir().unwrap();
        let range = range(SHA, SHA);

        let unwatched = changed_files(&store, &git(), data.path(), &range).await;
        watch_hello_world(&store).await;
        let unbound = changed_files(&store, &git(), data.path(), &range).await;

        assert_eq!(unwatched.unwrap_err().kind, "not_watched");
        assert_eq!(unbound.unwrap_err().kind, "unbound");
    }
}

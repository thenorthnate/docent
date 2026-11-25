//! Cloning a watched repo and binding the result (PLAN §4, "Repo binding").

use std::path::Path;

use docent_git::{CloneProgress, Git};
use docent_store::{Store, WatchedRepo};

use crate::bind::storable_path;
use crate::command_error::CommandError;

/// A clone's progress, as the UI shows it. Until git reports a percentage
/// there is none.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CloneUpdate {
    phase: String,
    percent: Option<u8>,
}

impl CloneUpdate {
    /// Sent once a folder is picked, before git has anything to report.
    pub fn starting() -> Self {
        Self {
            phase: "Starting".to_string(),
            percent: None,
        }
    }
}

impl From<CloneProgress> for CloneUpdate {
    fn from(progress: CloneProgress) -> Self {
        Self {
            phase: progress.phase,
            percent: Some(progress.percent),
        }
    }
}

/// Clones watched `repo` into a new folder in `parent` and binds it. Nothing
/// is stored unless the clone succeeds.
pub async fn clone(
    store: &Store,
    git: &Git,
    repo: &WatchedRepo,
    parent: &Path,
    mut on_update: impl FnMut(CloneUpdate),
) -> Result<WatchedRepo, CommandError> {
    on_update(CloneUpdate::starting());
    let clone = git
        .clone_repo(parent, &repo.host, &repo.owner, &repo.name, |progress| {
            on_update(progress.into());
        })
        .await?;
    Ok(store.bind_repo(repo.id, storable_path(&clone)?).await?)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use docent_store::NewRepo;

    async fn watched_hello_world(store: &Store) -> WatchedRepo {
        let account = store.upsert_account("github.com", "octocat").await.unwrap();
        let repo = NewRepo {
            owner: "octocat".into(),
            name: "Hello-World".into(),
            default_branch: "main".into(),
        };
        store.watch_repo(account, &repo).await.unwrap()
    }

    /// Git with `home` as its home, where ssh runs `ssh` and serves github.com.
    fn git_over(home: &Path, ssh: &str) -> Git {
        std::fs::write(home.join("ssh.sh"), format!("{ssh}\n")).unwrap();
        let config = format!(
            "[url \"git@github.com:\"]\n\tinsteadOf = https://github.com/\n\
             [core]\n\tsshCommand = sh '{}'\n[ssh]\n\tvariant = simple\n",
            home.join("ssh.sh").display()
        );
        std::fs::write(home.join(".gitconfig"), config).unwrap();
        let path = std::env::var_os("PATH").expect("tests run with a PATH");
        Git::with_env(path, [("HOME".into(), home.into())])
    }

    #[tokio::test]
    async fn a_finished_clone_is_bound() {
        let store = Store::open_in_memory().await.unwrap();
        let repo = watched_hello_world(&store).await;
        let dir = tempfile::tempdir().unwrap();
        let forge = dir.path().join("forge.git");
        let made = std::process::Command::new("git")
            .args(["init", "--quiet", "--bare"])
            .arg(&forge)
            .status();
        assert!(made.unwrap().success());
        let git = git_over(
            dir.path(),
            &format!("exec git upload-pack '{}'", forge.display()),
        );
        let parent = tempfile::tempdir().unwrap();
        let parent = parent.path().canonicalize().unwrap();
        let mut updates = Vec::new();

        let bound = clone(&store, &git, &repo, &parent, |u| updates.push(u))
            .await
            .expect("clones");

        let expected = parent.join("Hello-World");
        assert_eq!(bound.local_path.as_deref(), expected.to_str());
        assert_eq!(store.watched_repo(repo.id).await.unwrap(), bound);
        assert_eq!(updates.first(), Some(&CloneUpdate::starting()));
    }

    #[tokio::test]
    async fn a_failed_clone_leaves_the_repo_unbound() {
        let store = Store::open_in_memory().await.unwrap();
        let repo = watched_hello_world(&store).await;
        let home = tempfile::tempdir().unwrap();
        let git = git_over(home.path(), "exit 255");
        let parent = tempfile::tempdir().unwrap();

        let err = clone(&store, &git, &repo, parent.path(), |_| {})
            .await
            .unwrap_err();

        assert_eq!(err.kind, "git_failed");
        assert_eq!(store.watched_repo(repo.id).await.unwrap().local_path, None);
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
    }

    #[test]
    fn updates_reach_the_ui_as_phase_and_percent() {
        let update = CloneUpdate::from(CloneProgress {
            phase: "Receiving objects".into(),
            percent: 45,
        });

        assert_eq!(
            serde_json::to_value(update).unwrap(),
            serde_json::json!({ "phase": "Receiving objects", "percent": 45 })
        );
    }
}

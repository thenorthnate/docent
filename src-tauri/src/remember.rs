//! Remembering connected hosts across launches: host name and login go in
//! the store, and the token is read from `gh` again at startup.

use std::future::Future;

use docent_forge::{AccountSummary, Accounts, GhCli};
use docent_store::Store;

use crate::command_error::CommandError;

/// Connects `host` and remembers it. Only the host and login reach the
/// store: it has no column, and no API, that could take a token.
pub async fn connect(
    accounts: &Accounts,
    store: &Store,
    host: &str,
    gh: &GhCli,
) -> Result<AccountSummary, CommandError> {
    let summary = accounts.connect(host, gh).await?;
    store.upsert_account(&summary.host, &summary.login).await?;
    Ok(summary)
}

/// Reconnects every remembered host. A host that fails stays listed with its
/// error. `gh` is only asked for (which reads the login shell's `PATH`) when
/// there is a host to reconnect.
///
/// Returns an error only if the store can't be read or written; a host that
/// fails to reconnect is not an error here.
pub async fn restore_all<F, Fut>(
    accounts: &Accounts,
    store: &Store,
    gh: F,
) -> Result<(), CommandError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = GhCli>,
{
    let saved = store.saved_accounts().await?;
    if saved.is_empty() {
        return Ok(());
    }
    let gh = gh().await;
    // Every host is tried even if saving one fails; the first failure is
    // reported once all have been tried.
    let mut first_error = None;
    for account in saved {
        // The login may have changed if the user switched `gh` accounts.
        if let Ok(summary) = accounts.restore(&account.host, &account.login, &gh).await
            && summary.login != account.login
            && let Err(err) = store.upsert_account(&summary.host, &summary.login).await
        {
            first_error.get_or_insert(err);
        }
    }
    first_error.map_or(Ok(()), |err| Err(err.into()))
}

#[cfg(all(test, unix))]
mod tests {
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use docent_store::SavedAccount;
    use tempfile::TempDir;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const TOKEN: &str = "gho_remember_me_not";

    /// A temp dir holding a fake `gh` that prints `token` for github.com, or
    /// fails as if logged out when `token` is `None`.
    fn fake_gh(token: Option<&str>) -> (TempDir, GhCli) {
        let dir = tempfile::tempdir().expect("temp dir");
        let body = match token {
            Some(token) => format!("printf '%s\\n' '{token}'"),
            None => "echo 'no oauth token' >&2; exit 1".to_string(),
        };
        let gh = dir.path().join("gh");
        std::fs::write(&gh, format!("#!/bin/sh\n{body}\n")).expect("write fake gh");
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let cli = GhCli::new(OsString::from(dir.path()));
        (dir, cli)
    }

    /// A GitHub API that answers `GET /user` with `login` for [`TOKEN`].
    async fn github(login: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user"))
            .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "login": login, "id": 1
            })))
            .mount(&server)
            .await;
        server
    }

    fn accounts_against(server: &MockServer) -> Accounts {
        let base = server.uri();
        Accounts::with_api_base(move |_| base.clone())
    }

    fn saved(host: &str, login: &str) -> SavedAccount {
        SavedAccount {
            host: host.to_string(),
            login: login.to_string(),
        }
    }

    #[tokio::test]
    async fn a_connected_host_reconnects_after_a_restart() {
        let api = github("octocat").await;
        let (_dir, gh) = fake_gh(Some(TOKEN));
        let store = Store::open_in_memory().await.unwrap();
        connect(&accounts_against(&api), &store, "github.com", &gh)
            .await
            .expect("connect");

        // A restart: the store remains, the in-memory accounts don't.
        let after_restart = accounts_against(&api);
        restore_all(&after_restart, &store, || async { gh.clone() })
            .await
            .unwrap();

        let listed = after_restart.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            (listed[0].host.as_str(), listed[0].login.as_str()),
            ("github.com", "octocat")
        );
        assert_eq!(listed[0].error, None);
    }

    #[tokio::test]
    async fn a_host_that_fails_to_reconnect_is_listed_and_still_remembered() {
        let api = github("octocat").await;
        let store = Store::open_in_memory().await.unwrap();
        store.upsert_account("github.com", "octocat").await.unwrap();
        let (_dir, logged_out) = fake_gh(None);

        let accounts = accounts_against(&api);
        restore_all(&accounts, &store, || async { logged_out })
            .await
            .unwrap();

        let listed = accounts.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].error.as_ref().map(|e| e.kind()),
            Some("not_logged_in")
        );
        assert_eq!(
            store.saved_accounts().await.unwrap(),
            vec![saved("github.com", "octocat")],
            "a failed reconnect must not forget the host"
        );
    }

    #[tokio::test]
    async fn a_changed_login_is_remembered_on_reconnect() {
        let api = github("hubot").await;
        let store = Store::open_in_memory().await.unwrap();
        store.upsert_account("github.com", "octocat").await.unwrap();
        let (_dir, gh) = fake_gh(Some(TOKEN));

        restore_all(&accounts_against(&api), &store, || async { gh })
            .await
            .unwrap();

        assert_eq!(
            store.saved_accounts().await.unwrap(),
            vec![saved("github.com", "hubot")]
        );
    }

    #[tokio::test]
    async fn with_nothing_remembered_gh_is_never_asked_for() {
        let store = Store::open_in_memory().await.unwrap();
        let accounts = Accounts::with_api_base(|_| unreachable!("nothing to reconnect"));

        restore_all(&accounts, &store, || async {
            unreachable!("gh (and the login shell) must not be touched")
        })
        .await
        .unwrap();

        assert_eq!(accounts.list(), vec![]);
    }

    // ---- Denial test: the token never reaches the disk. ----

    /// Every byte of every file in `dir`: the database and any journal or
    /// write-ahead log beside it.
    fn all_bytes(dir: &Path) -> Vec<u8> {
        std::fs::read_dir(dir)
            .expect("read dir")
            .flat_map(|entry| std::fs::read(entry.expect("entry").path()).expect("read"))
            .collect()
    }

    #[tokio::test]
    async fn the_stored_account_rows_contain_no_token() {
        let api = github("octocat").await;
        let (_gh_dir, gh) = fake_gh(Some(TOKEN));
        let db_dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(&db_dir.path().join("docent.sqlite"))
            .await
            .unwrap();

        let accounts = accounts_against(&api);
        connect(&accounts, &store, "github.com", &gh)
            .await
            .expect("connect");
        restore_all(&accounts, &store, || async { gh.clone() })
            .await
            .unwrap();

        // Check while open (journal files present) and after closing.
        let needle = TOKEN.as_bytes();
        let open_bytes = all_bytes(db_dir.path());
        assert!(
            !open_bytes.windows(needle.len()).any(|w| w == needle),
            "token on disk"
        );
        store.close().await;
        let closed_bytes = all_bytes(db_dir.path());
        assert!(
            closed_bytes
                .windows(b"octocat".len())
                .any(|w| w == b"octocat"),
            "the login should be on disk, or this test isn't looking in the right place"
        );
        assert!(
            !closed_bytes.windows(needle.len()).any(|w| w == needle),
            "token on disk"
        );
    }
}

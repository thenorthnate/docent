//! The login shell's `PATH`. Apps launched from Finder get a minimal `PATH`
//! that usually lacks Homebrew, so `gh` (and later `git`) would not be found.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

/// Asks `shell` for its `PATH` as an interactive login shell would set it.
/// Returns `None` if the shell fails, hangs past `timeout`, or prints no
/// `PATH`.
pub async fn login_shell_path(shell: &Path, timeout: Duration) -> Option<OsString> {
    // Interactive (`-i`) as well as login (`-l`), because many users set PATH
    // in `.zshrc`. The markers separate the value from anything rc files print.
    let script = format!("printf '%s' \"{MARKER}${{PATH}}{MARKER}\"");
    let run = tokio::process::Command::new(shell)
        .args(["-ilc", &script])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(timeout, run).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let path = stdout.split(MARKER).nth(1)?;
    // Relative entries (`.`, `bin`, an empty entry) would resolve against the
    // app's working directory, so only absolute ones are kept.
    let dirs: Vec<_> = std::env::split_paths(path)
        .filter(|dir| dir.is_absolute())
        .collect();
    if dirs.is_empty() {
        return None;
    }
    std::env::join_paths(dirs).ok()
}

const MARKER: &str = "__DOCENT_PATH__";

/// Where Homebrew installs `gh` on Apple silicon and on Intel Macs.
const HOMEBREW_DIRS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];

/// Used when the login shell can't be asked: the app's own `PATH` with the
/// Homebrew directories added, so a standard `gh` install is still found.
pub fn fallback_path(current: Option<OsString>) -> OsString {
    let mut dirs: Vec<std::path::PathBuf> = current
        .as_deref()
        .map(|path| std::env::split_paths(path).collect())
        .unwrap_or_default();
    for dir in HOMEBREW_DIRS {
        if !dirs.iter().any(|d| d == Path::new(dir)) {
            dirs.push(dir.into());
        }
    }
    // Only fails on a `:` inside a directory name, which split_paths and the
    // constants above can't produce.
    std::env::join_paths(dirs).unwrap_or_default()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    const SECOND: Duration = Duration::from_secs(1);

    fn fake_shell(dir: &tempfile::TempDir, body: &str) -> PathBuf {
        let shell = dir.path().join("fake-shell");
        std::fs::write(&shell, format!("#!/bin/sh\n{body}")).expect("write fake shell");
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        shell
    }

    #[test]
    fn fallback_adds_homebrew_dirs_once() {
        assert_eq!(
            fallback_path(Some("/usr/bin:/usr/local/bin".into())),
            OsString::from("/usr/bin:/usr/local/bin:/opt/homebrew/bin")
        );
        assert_eq!(
            fallback_path(None),
            OsString::from("/opt/homebrew/bin:/usr/local/bin")
        );
    }

    #[tokio::test]
    async fn reads_path_from_a_real_login_shell() {
        let path = login_shell_path(Path::new("/bin/sh"), Duration::from_secs(10))
            .await
            .expect("sh reports a PATH");

        let path = path.to_string_lossy().into_owned();
        assert!(path.split(':').any(|dir| dir == "/usr/bin"), "{path}");
    }

    /// rc files often print banners; only the marked value counts.
    #[tokio::test]
    async fn ignores_whatever_rc_files_print_around_the_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let shell = fake_shell(
            &dir,
            "echo 'Welcome!'\nPATH=/opt/homebrew/bin:/usr/bin\nexport PATH\nshift\nexec /bin/sh -c \"$1\"\n",
        );

        let path = login_shell_path(&shell, SECOND).await;

        assert_eq!(path, Some(OsString::from("/opt/homebrew/bin:/usr/bin")));
    }

    /// A relative entry would make `gh` resolve against the app's working
    /// directory, so only absolute directories are kept.
    #[tokio::test]
    async fn drops_relative_path_entries() {
        let dir = tempfile::tempdir().expect("temp dir");
        let shell = fake_shell(
            &dir,
            "PATH=.:bin:/usr/bin::./x:/opt/homebrew/bin\nexport PATH\nshift\nexec /bin/sh -c \"$1\"\n",
        );

        let path = login_shell_path(&shell, SECOND).await;

        assert_eq!(path, Some(OsString::from("/usr/bin:/opt/homebrew/bin")));
    }

    #[tokio::test]
    async fn a_failing_or_silent_shell_gives_none() {
        let dir = tempfile::tempdir().expect("temp dir");

        assert_eq!(
            login_shell_path(&fake_shell(&dir, "exit 1\n"), SECOND).await,
            None
        );
        assert_eq!(
            login_shell_path(&dir.path().join("missing"), SECOND).await,
            None
        );
    }

    #[tokio::test]
    async fn a_hanging_shell_gives_none_after_the_timeout() {
        let dir = tempfile::tempdir().expect("temp dir");
        let shell = fake_shell(&dir, "exec sleep 30\n");

        let started = std::time::Instant::now();
        let path = login_shell_path(&shell, Duration::from_millis(200)).await;

        assert_eq!(path, None);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

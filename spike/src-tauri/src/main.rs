//! Docent M0 viewer spike. Throwaway code: see spike/README.md.
//!
//! The Rust side only feeds real `git diff` hunks to the UI and saves benchmark
//! results. Everything being compared lives in ../ui.

use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

const MAX_FILES: usize = 300;
const MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Hunk {
    old_start: u32,
    old_lines: u32,
    new_start: u32,
    new_lines: u32,
}

#[derive(Serialize)]
struct FileDiff {
    path: String,
    original: String,
    modified: String,
    hunks: Vec<Hunk>,
}

fn git(repo: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.quotepath=off"])
        .args(args)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(out.stdout)
}

fn show(repo: &str, rev: &str, path: &str) -> Result<String, String> {
    let bytes = git(repo, &["show", &format!("{rev}:{path}")])?;
    if bytes.len() > MAX_BYTES {
        return Err(format!("{path} is larger than {MAX_BYTES} bytes"));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Parses `@@ -a[,b] +c[,d] @@`.
fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let mut parts = line.strip_prefix("@@ ")?.split(' ');
    let range = |s: &str| -> Option<(u32, u32)> {
        let mut it = s[1..].splitn(2, ',');
        let start = it.next()?.parse().ok()?;
        let count = it.next().map_or(Some(1), |c| c.parse().ok())?;
        Some((start, count))
    };
    let (old_start, old_lines) = range(parts.next()?)?;
    let (new_start, new_lines) = range(parts.next()?)?;
    Some(Hunk { old_start, old_lines, new_start, new_lines })
}

/// File paths and hunks from `git diff -U0` output. Binary files are skipped.
fn parse_diff(text: &str) -> Vec<(String, bool, bool, Vec<Hunk>)> {
    let mut files = Vec::new();
    let mut cur: Option<(String, bool, bool, Vec<Hunk>)> = None;
    let mut old_path = String::new();
    for line in text.lines() {
        if line.starts_with("diff --git ") {
            files.extend(cur.take());
        } else if let Some(p) = line.strip_prefix("--- ") {
            // git appends a tab to paths that contain spaces.
            old_path = p.trim_end_matches('\t').to_string();
        } else if let Some(p) = line.strip_prefix("+++ ") {
            let p = p.trim_end_matches('\t');
            let added = old_path == "/dev/null";
            let deleted = p == "/dev/null";
            let path = if deleted { &old_path[2..] } else { &p[2..] };
            cur = Some((path.to_string(), added, deleted, Vec::new()));
        } else if line.starts_with("@@ ") {
            if let (Some(c), Some(h)) = (cur.as_mut(), parse_hunk_header(line)) {
                c.3.push(h);
            }
        }
    }
    files.extend(cur);
    files
}

#[tauri::command]
async fn load_git_diff(repo: String, base: String, head: String) -> Result<Vec<FileDiff>, String> {
    let raw = git(&repo, &["diff", "--no-color", "--no-ext-diff", "--no-textconv", "--no-renames", "-U0", &base, &head])?;
    let mut out = Vec::new();
    for (path, added, deleted, hunks) in parse_diff(&String::from_utf8_lossy(&raw)).into_iter().take(MAX_FILES) {
        // Oversized or unreadable files are skipped rather than failing the whole diff.
        let read = |rev: &str, skip: bool| if skip { Ok(String::new()) } else { show(&repo, rev, &path) };
        match (read(&base, added), read(&head, deleted)) {
            (Ok(original), Ok(modified)) => out.push(FileDiff { path, original, modified, hunks }),
            (Err(e), _) | (_, Err(e)) => eprintln!("skipping {path}: {e}"),
        }
    }
    Ok(out)
}

#[tauri::command]
fn save_results(json: String) -> Result<String, String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../results");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let path = dir.join(format!("bench-{}-{secs}.json", std::env::consts::OS));
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.canonicalize().unwrap_or(path).display().to_string())
}

/// `SPIKE_BENCH=1` runs the benchmark on start-up and quits when it's done.
#[tauri::command]
fn bench_requested() -> bool {
    std::env::var("SPIKE_BENCH").is_ok_and(|v| v == "1")
}

/// `SPIKE_GIT="<repo>|<base>|<head>"` adds that real diff to the benchmark (hunk agreement).
#[tauri::command]
fn bench_git_target() -> Option<Vec<String>> {
    let v = std::env::var("SPIKE_GIT").ok()?;
    let parts: Vec<String> = v.split('|').map(str::to_string).collect();
    (parts.len() == 3).then_some(parts)
}

/// Mirrors the UI's event log to stdout, for headless benchmark runs.
#[tauri::command]
fn log_line(line: String) {
    println!("{line}");
}

#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![load_git_diff, save_results, bench_requested, bench_git_target, log_line, quit])
        .run(tauri::generate_context!())
        .expect("tauri app failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hunk_headers_with_and_without_counts() {
        assert_eq!(parse_hunk_header("@@ -3,0 +4,2 @@ fn x()"), Some(Hunk { old_start: 3, old_lines: 0, new_start: 4, new_lines: 2 }));
        assert_eq!(parse_hunk_header("@@ -7 +7 @@"), Some(Hunk { old_start: 7, old_lines: 1, new_start: 7, new_lines: 1 }));
    }

    #[test]
    fn parses_added_deleted_and_binary_files() {
        let text = "diff --git a/new.ts b/new.ts\nnew file mode 100644\n--- /dev/null\n+++ b/new.ts\n@@ -0,0 +1,2 @@\n+a\n+b\n\
diff --git a/img.png b/img.png\nBinary files a/img.png and b/img.png differ\n\
diff --git a/old.ts b/old.ts\n--- a/old.ts\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        let files = parse_diff(text);
        assert_eq!(files.len(), 2);
        assert_eq!((files[0].0.as_str(), files[0].1, files[0].2), ("new.ts", true, false));
        assert_eq!((files[1].0.as_str(), files[1].1, files[1].2), ("old.ts", false, true));
        assert_eq!(files[1].3, vec![Hunk { old_start: 1, old_lines: 1, new_start: 0, new_lines: 0 }]);
    }

    #[test]
    fn strips_the_tab_git_adds_after_paths_with_spaces() {
        let text = "diff --git a/my file.ts b/my file.ts\n--- a/my file.ts\t\n+++ b/my file.ts\t\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(parse_diff(text)[0].0, "my file.ts");
    }
}

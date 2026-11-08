//! Worktree checkout and plain diff for one PR, plus blame/log context for Explain.

use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const MAX_DIFF_BYTES: usize = 3 * 1024 * 1024;

async fn git<I, S>(dir: &Path, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<_> = args.into_iter().map(|a| a.as_ref().to_owned()).collect();
    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        // No repo or user hooks run from Docent's git calls (e.g. post-checkout on worktree add).
        .args(["-c", "core.hooksPath=/dev/null"])
        .args(&args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .kill_on_drop(true);
    for k in crate::TOKEN_VARS {
        cmd.env_remove(k);
    }
    let out = cmd
        .output()
        .await
        .map_err(|e| format!("running git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.iter()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Checkout {
    pub worktree: String,
    pub head_ref: String,
    pub base_ref: String,
    pub head_sha: String,
    pub head_matches_api: bool,
    pub diff: String,
    pub truncated: bool,
}

pub fn refs_for(number: u64) -> (String, String) {
    (
        format!("refs/docent/pr/{number}/head"),
        format!("refs/docent/pr/{number}/base"),
    )
}

pub async fn checkout_pr(
    repo: &Path,
    wt_root: &Path,
    number: u64,
    base_branch: &str,
    expected_head: &str,
) -> Result<Checkout, String> {
    let (head_ref, base_ref) = refs_for(number);
    git(
        repo,
        [
            "fetch",
            "--no-tags",
            "origin",
            &format!("+refs/pull/{number}/head:{head_ref}"),
            &format!("+refs/heads/{base_branch}:{base_ref}"),
        ],
    )
    .await?;
    let head_sha = git(repo, ["rev-parse", &head_ref])
        .await?
        .trim()
        .to_string();

    std::fs::create_dir_all(wt_root).map_err(|e| e.to_string())?;
    let name = repo
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".into());
    let wt = wt_root.join(format!("{name}-pr-{number}"));
    if wt.exists() {
        let _ = git(
            repo,
            [
                OsStr::new("worktree"),
                OsStr::new("remove"),
                OsStr::new("--force"),
                wt.as_os_str(),
            ],
        )
        .await;
        if wt.exists() {
            std::fs::remove_dir_all(&wt).map_err(|e| e.to_string())?;
        }
    }
    git(repo, ["worktree", "prune"]).await?;
    git(
        repo,
        [
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("--detach"),
            wt.as_os_str(),
            OsStr::new(&head_ref),
        ],
    )
    .await?;

    let mut diff = git(
        &wt,
        [
            "diff",
            "--no-color",
            "--no-ext-diff",
            &format!("{base_ref}...{head_ref}"),
        ],
    )
    .await?;
    let truncated = diff.len() > MAX_DIFF_BYTES;
    if truncated {
        let mut cut = MAX_DIFF_BYTES;
        while !diff.is_char_boundary(cut) {
            cut -= 1;
        }
        diff.truncate(cut);
    }
    Ok(Checkout {
        worktree: wt.to_string_lossy().into_owned(),
        head_matches_api: expected_head.is_empty() || head_sha == expected_head,
        head_ref,
        base_ref,
        head_sha,
        diff,
        truncated,
    })
}

#[tauri::command]
pub async fn checkout_pr_cmd(
    app: AppHandle,
    repo_path: String,
    number: u64,
    base_branch: String,
    head_sha: String,
) -> Result<Checkout, String> {
    let wt_root = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("worktrees");
    checkout_pr(
        &PathBuf::from(repo_path),
        &wt_root,
        number,
        &base_branch,
        &head_sha,
    )
    .await
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HunkContext {
    pub blame: String,
    pub log: String,
    pub commits: Vec<String>,
}

/// Blame of the hunk's *old* lines at the PR base (why the code was there before),
/// plus recent history of the file.
#[tauri::command]
pub async fn hunk_context(
    worktree: String,
    number: u64,
    file: String,
    old_file: String,
    old_start: u64,
    old_count: u64,
) -> Result<HunkContext, String> {
    let wt = PathBuf::from(worktree);
    let (_, base_ref) = refs_for(number);
    let mut blame = String::new();
    let mut commits: Vec<String> = vec![];
    if old_count > 0 && old_file != "/dev/null" {
        let range = format!("{old_start},+{old_count}");
        blame = git(
            &wt,
            [
                "blame",
                "--root",
                "-l",
                "--date=short",
                "-L",
                &range,
                &base_ref,
                "--",
                &old_file,
            ],
        )
        .await
        .unwrap_or_else(|e| format!("(blame failed: {e})"));
        for line in blame.lines() {
            let sha = line
                .trim_start_matches('^')
                .split_whitespace()
                .next()
                .unwrap_or("");
            if sha.len() >= 40 && !sha.starts_with("0000000") && !commits.iter().any(|c| c == sha) {
                commits.push(sha.to_string());
            }
        }
    }
    let log = git(
        &wt,
        [
            "log",
            "-n",
            "8",
            "--date=short",
            "--format=%h %ad %an %s",
            &base_ref,
            "--",
            &file,
        ],
    )
    .await
    .unwrap_or_else(|e| format!("(log failed: {e})"));
    Ok(HunkContext {
        blame,
        log,
        commits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn sh(dir: &Path, args: &[&str]) {
        let st = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .expect("git runs");
        assert!(st.success(), "git {args:?}");
    }

    #[tokio::test]
    async fn checks_out_pr_head_into_worktree_and_diffs_against_base() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let origin = tmp.path().join("origin");
        std::fs::create_dir(&origin).unwrap();
        sh(&origin, &["init", "-q", "-b", "main"]);
        std::fs::write(origin.join("a.txt"), "one\ntwo\n").unwrap();
        sh(&origin, &["add", "."]);
        sh(&origin, &["commit", "-qm", "base"]);
        sh(&origin, &["checkout", "-qb", "feature"]);
        std::fs::write(origin.join("a.txt"), "one\nTWO\n").unwrap();
        sh(&origin, &["commit", "-qam", "change"]);
        sh(&origin, &["update-ref", "refs/pull/7/head", "feature"]);
        sh(&origin, &["checkout", "-q", "main"]);

        let clone = tmp.path().join("clone");
        sh(
            tmp.path(),
            &[
                "clone",
                "-q",
                origin.to_str().unwrap(),
                clone.to_str().unwrap(),
            ],
        );

        let wt_root = tmp.path().join("wts");
        let c = checkout_pr(&clone, &wt_root, 7, "main", "")
            .await
            .expect("checkout");
        assert!(
            c.diff.contains("-two") && c.diff.contains("+TWO"),
            "{}",
            c.diff
        );
        assert_eq!(
            std::fs::read_to_string(Path::new(&c.worktree).join("a.txt")).unwrap(),
            "one\nTWO\n"
        );

        // Re-checkout replaces the worktree cleanly.
        checkout_pr(&clone, &wt_root, 7, "main", &c.head_sha)
            .await
            .expect("second checkout");

        let ctx = hunk_context(c.worktree.clone(), 7, "a.txt".into(), "a.txt".into(), 1, 2)
            .await
            .expect("context");
        assert_eq!(ctx.commits.len(), 1, "{}", ctx.blame);
        assert!(ctx.log.contains("base"));
    }
}

//! GitHub access with the token from `gh auth token`. Answers Q6: which calls work under SSO.

use crate::AppState;
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::State;

#[derive(Clone)]
pub struct Session {
    host: String,
    token: String,
    http: reqwest::Client,
}

/// Response headers worth recording for the write-up. Never includes Authorization.
const INTERESTING_HEADERS: &[&str] = &[
    "x-oauth-scopes",
    "x-accepted-oauth-scopes",
    "x-github-sso",
    "x-poll-interval",
    "last-modified",
    "x-ratelimit-resource",
    "x-ratelimit-remaining",
];

#[derive(Serialize, Debug)]
pub struct ProbeResult {
    pub name: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub ok: bool,
    pub headers: Vec<(String, String)>,
    pub note: String,
}

impl Session {
    pub async fn from_gh(host: &str) -> Result<Self, String> {
        let mut cmd = tokio::process::Command::new("gh");
        cmd.args(["auth", "token", "--hostname", host]);
        for k in crate::TOKEN_VARS {
            // Make sure gh reports its *stored* token, not one inherited from this shell.
            cmd.env_remove(k);
        }
        let out = cmd
            .output()
            .await
            .map_err(|e| format!("running gh: {e} (is gh on PATH?)"))?;
        if !out.status.success() {
            return Err(format!(
                "gh auth token failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if token.is_empty() {
            return Err("gh returned an empty token".into());
        }
        let http = reqwest::Client::builder()
            .user_agent("docent-spike/0.0")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Session {
            host: host.to_string(),
            token,
            http,
        })
    }

    /// Token type prefix (gho_, ghp_, github_pat_…), safe to show.
    pub fn token_kind(&self) -> String {
        match self.token.find('_') {
            Some(i) if i < 12 => self.token[..=i].to_string(),
            _ => "(unknown prefix)".into(),
        }
    }

    fn api_base(&self) -> String {
        if self.host == "github.com" {
            "https://api.github.com".into()
        } else if self.host.ends_with(".ghe.com") {
            format!("https://api.{}", self.host)
        } else {
            format!("https://{}/api/v3", self.host)
        }
    }

    fn graphql_url(&self) -> String {
        if self.host == "github.com" {
            "https://api.github.com/graphql".into()
        } else if self.host.ends_with(".ghe.com") {
            format!("https://api.{}/graphql", self.host)
        } else {
            format!("https://{}/api/graphql", self.host)
        }
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.api_base(), path)
        }
    }

    fn req(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, self.url(path))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    async fn run(
        &self,
        name: &str,
        method: Method,
        path: &str,
        body: Option<Value>,
        extra_headers: &[(&str, String)],
    ) -> (ProbeResult, Option<Value>) {
        let mut rb = self.req(method.clone(), path);
        for (k, v) in extra_headers {
            rb = rb.header(*k, v);
        }
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        let mut result = ProbeResult {
            name: name.into(),
            method: method.to_string(),
            url: self.url(path),
            status: None,
            ok: false,
            headers: vec![],
            note: String::new(),
        };
        let resp = match rb.send().await {
            Ok(r) => r,
            Err(e) => {
                result.note = format!("request error: {e}");
                return (result, None);
            }
        };
        let status = resp.status();
        result.status = Some(status.as_u16());
        for h in INTERESTING_HEADERS {
            if let Some(v) = resp.headers().get(*h) {
                result
                    .headers
                    .push((h.to_string(), v.to_str().unwrap_or("?").to_string()));
            }
        }
        let text = resp.text().await.unwrap_or_default();
        let parsed: Option<Value> = serde_json::from_str(&text).ok();
        result.ok = status.is_success() || status == StatusCode::NOT_MODIFIED;
        if !status.is_success() && status != StatusCode::NOT_MODIFIED {
            result.note = text.chars().take(400).collect();
        }
        if let Some(errs) = parsed.as_ref().and_then(|v| v.get("errors")) {
            // GraphQL returns 200 with an errors array, e.g. for SAML enforcement.
            result.ok = false;
            result.note = errs.to_string().chars().take(400).collect();
        }
        (result, parsed)
    }
}

pub async fn probe(s: &Session, owner: &str, repo: &str) -> Vec<ProbeResult> {
    let mut out = vec![];
    let get = Method::GET;

    out.push(
        s.run("viewer (REST /user)", get.clone(), "/user", None, &[])
            .await
            .0,
    );
    out.push(
        s.run(
            "repo",
            get.clone(),
            &format!("/repos/{owner}/{repo}"),
            None,
            &[],
        )
        .await
        .0,
    );
    out.push(
        s.run(
            "open PRs",
            get.clone(),
            &format!("/repos/{owner}/{repo}/pulls?state=open&per_page=5"),
            None,
            &[],
        )
        .await
        .0,
    );
    out.push(
        s.run(
            "inbox search: review-requested:@me",
            get.clone(),
            "/search/issues?q=is%3Apr+is%3Aopen+review-requested%3A%40me&per_page=5",
            None,
            &[],
        )
        .await
        .0,
    );

    // Notifications: first call, then a conditional repeat that should come back 304.
    let (first, _) = s
        .run(
            "notifications",
            get.clone(),
            "/notifications?per_page=1",
            None,
            &[],
        )
        .await;
    let last_modified = first
        .headers
        .iter()
        .find(|(k, _)| k == "last-modified")
        .map(|(_, v)| v.clone());
    out.push(first);
    if let Some(lm) = last_modified {
        let (mut second, _) = s
            .run(
                "notifications (If-Modified-Since, expect 304)",
                get.clone(),
                "/notifications?per_page=1",
                None,
                &[("If-Modified-Since", lm)],
            )
            .await;
        if second.status == Some(304) {
            second.note = "304 as expected".into();
        }
        out.push(second);
    } else if let Some(first) = out.last_mut() {
        first.note = "no Last-Modified header (empty inbox?): 304 check skipped".into();
    }

    // Blame → PR lookup: PRs associated with the default branch's latest commit.
    let (commits, body) = s
        .run(
            "latest commit",
            get.clone(),
            &format!("/repos/{owner}/{repo}/commits?per_page=1"),
            None,
            &[],
        )
        .await;
    out.push(commits);
    if let Some(sha) = body
        .as_ref()
        .and_then(|b| b.get(0))
        .and_then(|c| c.get("sha"))
        .and_then(Value::as_str)
    {
        out.push(
            s.run(
                "PRs for commit (blame → PR)",
                get.clone(),
                &format!("/repos/{owner}/{repo}/commits/{sha}/pulls"),
                None,
                &[],
            )
            .await
            .0,
        );
    }

    let query = r#"query($owner:String!,$name:String!){
      viewer{login}
      rateLimit{remaining resetAt}
      repository(owner:$owner,name:$name){
        pullRequests(first:5,states:OPEN,orderBy:{field:UPDATED_AT,direction:DESC}){
          nodes{number title isDraft reviewDecision
            reviews(last:1){nodes{author{login} commit{oid}}}
            commits(last:1){nodes{commit{statusCheckRollup{state}}}}}}}}"#;
    out.push(
        s.run(
            "GraphQL: PRs, reviews, checks",
            Method::POST,
            &s.graphql_url(),
            Some(json!({"query": query, "variables": {"owner": owner, "name": repo}})),
            &[],
        )
        .await
        .0,
    );
    out
}

async fn session(state: &State<'_, AppState>) -> Result<Session, String> {
    state
        .gh
        .lock()
        .await
        .clone()
        .ok_or_else(|| "not connected: click Connect first".to_string())
}

#[tauri::command]
pub async fn connect(state: State<'_, AppState>, host: String) -> Result<String, String> {
    let s = Session::from_gh(&host).await?;
    let msg = format!(
        "Got a token from gh for {host}, type {}. Held in memory only.",
        s.token_kind()
    );
    *state.gh.lock().await = Some(s);
    Ok(msg)
}

#[tauri::command]
pub async fn probe_cmd(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
) -> Result<Vec<ProbeResult>, String> {
    let s = session(&state).await?;
    Ok(probe(&s, &owner, &repo).await)
}

#[tauri::command]
pub async fn list_prs(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
) -> Result<Vec<Value>, String> {
    let s = session(&state).await?;
    let (r, body) = s
        .run(
            "list",
            Method::GET,
            &format!("/repos/{owner}/{repo}/pulls?state=open&per_page=50"),
            None,
            &[],
        )
        .await;
    if !r.ok {
        return Err(format!("{:?}: {}", r.status, r.note));
    }
    let prs = body.and_then(|b| b.as_array().cloned()).unwrap_or_default();
    Ok(prs
        .iter()
        .map(|p| {
            json!({
                "number": p["number"],
                "title": p["title"],
                "body": p["body"],
                "author": p["user"]["login"],
                "draft": p["draft"],
                "headRef": p["head"]["ref"],
                "headSha": p["head"]["sha"],
                "baseRef": p["base"]["ref"],
                "url": p["html_url"],
            })
        })
        .collect())
}

#[tauri::command]
pub async fn prs_for_commit(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    sha: String,
) -> Result<Vec<Value>, String> {
    let s = session(&state).await?;
    let (r, body) = s
        .run(
            "prs for commit",
            Method::GET,
            &format!("/repos/{owner}/{repo}/commits/{sha}/pulls"),
            None,
            &[],
        )
        .await;
    if !r.ok {
        return Err(format!("{:?}: {}", r.status, r.note));
    }
    let prs = body.and_then(|b| b.as_array().cloned()).unwrap_or_default();
    Ok(prs
        .iter()
        .map(|p| json!({"number": p["number"], "title": p["title"], "body": p["body"]}))
        .collect())
}

/// Creates a PENDING review (visible only to you) and deletes it straight away.
/// Proves the token can write reviews without posting anything.
#[tauri::command]
pub async fn pending_review_roundtrip(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    number: u64,
) -> Result<String, String> {
    let s = session(&state).await?;
    let path = format!("/repos/{owner}/{repo}/pulls/{number}/reviews");
    let (created, body) = s
        .run(
            "create pending review",
            Method::POST,
            &path,
            Some(json!({"body": "docent spike: pending review test, deleted immediately"})),
            &[],
        )
        .await;
    if !created.ok {
        return Err(format!(
            "create failed: {:?} {} {:?}",
            created.status, created.note, created.headers
        ));
    }
    let id = body
        .as_ref()
        .and_then(|b| b.get("id"))
        .and_then(Value::as_u64)
        .ok_or("no review id in response")?;
    let (deleted, _) = s
        .run(
            "delete pending review",
            Method::DELETE,
            &format!("{path}/{id}"),
            None,
            &[],
        )
        .await;
    if !deleted.ok {
        return Err(format!(
            "created pending review {id} but delete failed ({:?}): delete it on GitHub",
            deleted.status
        ));
    }
    Ok(format!(
        "Created and deleted pending review {id}: review writes work."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live check against the current machine's gh login. Run with:
    /// DOCENT_SPIKE_REPO=owner/repo cargo test live_probe -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn live_probe() {
        let target = std::env::var("DOCENT_SPIKE_REPO").expect("set DOCENT_SPIKE_REPO=owner/repo");
        let host = std::env::var("DOCENT_SPIKE_HOST").unwrap_or_else(|_| "github.com".into());
        let (owner, repo) = target.split_once('/').expect("owner/repo");
        let s = Session::from_gh(&host).await.expect("gh token");
        println!("token kind: {}", s.token_kind());
        for r in probe(&s, owner, repo).await {
            println!(
                "{:<45} {:?} ok={} {:?} {}",
                r.name, r.status, r.ok, r.headers, r.note
            );
        }
    }

    #[test]
    fn enterprise_urls() {
        let mk = |host: &str| Session {
            host: host.into(),
            token: "gho_x".into(),
            http: reqwest::Client::new(),
        };
        assert_eq!(
            mk("github.com").graphql_url(),
            "https://api.github.com/graphql"
        );
        assert_eq!(mk("acme.ghe.com").api_base(), "https://api.acme.ghe.com");
        assert_eq!(
            mk("git.acme.corp").graphql_url(),
            "https://git.acme.corp/api/graphql"
        );
        assert_eq!(mk("github.com").token_kind(), "gho_");
    }
}

use std::path::{Path, PathBuf};

use crate::remote::RemoteRepo;
use crate::{Git, GitError};

/// A local clone checked against a forge repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundClone {
    /// The clone's top-level folder.
    pub path: PathBuf,
    /// The first remote that points at the repo, which PR heads are fetched
    /// from.
    pub remote: String,
}

impl Git {
    /// Checks that `folder` is in a clone of `host:owner/name`: one of its
    /// remotes must point at that repo. Returns the clone's top level.
    ///
    /// The folder isn't trusted, so this runs only `rev-parse` and `remote`,
    /// which run no hooks, fsmonitor or other repo-configured commands.
    pub async fn check_clone(
        &self,
        folder: &Path,
        host: &str,
        owner: &str,
        name: &str,
    ) -> Result<BoundClone, GitError> {
        let not_a_repo = || GitError::NotARepo(folder.display().to_string());
        // A missing working directory fails to spawn like a missing `git`
        // would, so rule it out first.
        if !folder.is_dir() {
            return Err(not_a_repo());
        }
        let top = match self.run(folder, &["rev-parse", "--show-toplevel"]).await {
            Ok(out) => PathBuf::from(out.trim_end_matches('\n')),
            Err(GitError::Failed(why)) if why.contains("not a git repository") => {
                return Err(not_a_repo());
            }
            Err(err) => return Err(err),
        };
        if top.as_os_str().is_empty() {
            return Err(not_a_repo());
        }

        let mut found = Vec::new();
        for remote in self.run(&top, &["remote"]).await?.lines() {
            // `get-url` applies `insteadOf` rewrites, as a fetch would.
            let urls = self
                .run(&top, &["remote", "get-url", "--all", remote])
                .await?;
            for url in urls.lines() {
                if RemoteRepo::parse(url).is_some_and(|r| r.is(host, owner, name)) {
                    return Ok(BoundClone {
                        path: top,
                        remote: remote.to_string(),
                    });
                }
                found.push(without_credentials(url));
            }
        }
        Err(GitError::NoMatchingRemote {
            repo: format!("{host}:{owner}/{name}"),
            found,
        })
    }
}

/// `url` without the `user:password@` of a URL with a scheme, so a token
/// embedded in a remote never reaches the UI.
pub(crate) fn without_credentials(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let authority_end = rest.find('/').unwrap_or(rest.len());
    match rest[..authority_end].rfind('@') {
        Some(at) => format!("{scheme}://{}", &rest[at + 1..]),
        None => url.to_string(),
    }
}

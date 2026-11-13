use crate::{AccountSummary, ConnectError};

/// A repo as `owner/name`, checked so that it can only ever be those two
/// URL path segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoName {
    owner: String,
    name: String,
}

impl RepoName {
    pub fn parse(_input: &str) -> Result<Self, RepoError> {
        todo!()
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A repo the forge confirmed exists, with its canonical spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeRepo {
    /// The connected account the repo was found with.
    pub account: AccountSummary,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
}

/// Why a repo could not be looked up. No message carries the token.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RepoError {
    #[error("{0:?} is not a repo. Enter it as owner/name, like octocat/Hello-World.")]
    InvalidName(String),
    #[error("{0} is not connected. Connect it in Settings → Accounts first.")]
    NotConnected(String),
    #[error("{host} has no repo {repo}, or your account can't see it.")]
    NotFound { host: String, repo: String },
    #[error(transparent)]
    Connect(#[from] ConnectError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_owner_slash_name() {
        let repo = RepoName::parse("  octocat/Hello-World ").unwrap();
        assert_eq!((repo.owner(), repo.name()), ("octocat", "Hello-World"));

        for ok in [
            "my-org/repo.name",
            "a/b",
            "user_corp/x_y",
            "org/.github",
            "o/-r",
        ] {
            assert!(RepoName::parse(ok).is_ok(), "{ok:?} should be accepted");
        }
    }

    /// Owner and name become URL path segments, so nothing that could add a
    /// segment, a query or a fragment may get through.
    #[test]
    fn rejects_anything_that_is_not_two_plain_segments() {
        let too_long = format!("o/{}", "n".repeat(101));
        for hostile in [
            "",
            "octocat",
            "/Hello-World",
            "octocat/",
            "octocat/Hello-World/issues",
            "octocat/..",
            "octocat/.",
            "../octocat",
            "-octocat/x",
            "octo cat/x",
            "octocat/Hello World",
            "octocat/x?y=1",
            "octocat/x#frag",
            "octocat/x%2Fy",
            "octocat\\x",
            "https://github.com/octocat/x",
            "octocat/x\n",
            "oçtocat/x",
            &too_long,
        ] {
            assert_eq!(
                RepoName::parse(hostile),
                Err(RepoError::InvalidName(hostile.to_string())),
                "{hostile:?} should be rejected"
            );
        }
    }
}

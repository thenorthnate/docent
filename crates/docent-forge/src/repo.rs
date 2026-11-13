use crate::{AccountSummary, ConnectError};

/// GitHub's longest login.
const MAX_OWNER_LEN: usize = 39;
/// GitHub's longest repo name.
const MAX_NAME_LEN: usize = 100;

/// A repo as `owner/name`, checked so that it can only ever be those two
/// URL path segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoName {
    owner: String,
    name: String,
}

impl RepoName {
    /// Accepts `owner/name` with surrounding spaces. The owner is up to 39
    /// ASCII letters, digits, `-` or `_` (GitHub's login rules, plus the `_`
    /// that Enterprise Managed Users have), not starting with `-`. The name
    /// is up to 100 ASCII letters, digits, `.`, `-` or `_`, and not `.` or
    /// `..`. That leaves no way to add a path segment, query or fragment.
    pub fn parse(input: &str) -> Result<Self, RepoError> {
        let invalid = || RepoError::InvalidName(input.to_string());
        let (owner, name) = input
            .trim_matches(' ')
            .split_once('/')
            .ok_or_else(invalid)?;

        let owner_ok = (1..=MAX_OWNER_LEN).contains(&owner.len())
            && !owner.starts_with('-')
            && owner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        let name_ok = (1..=MAX_NAME_LEN).contains(&name.len())
            && name != "."
            && name != ".."
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if !owner_ok || !name_ok {
            return Err(invalid());
        }
        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
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

impl RepoError {
    /// A stable name for the UI to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidName(_) => "invalid_name",
            Self::NotConnected(_) => "not_connected",
            Self::NotFound { .. } => "not_found",
            Self::Connect(err) => err.kind(),
        }
    }
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

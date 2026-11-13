//! GitHub repo names.

use crate::RepoError;

/// The longest GitHub login.
const MAX_OWNER_LEN: usize = 39;
/// The longest GitHub repo name.
const MAX_NAME_LEN: usize = 100;

/// A GitHub repo as `owner/name`, checked against GitHub's naming rules so
/// that it can only ever be those two URL path segments. Other forges name
/// repos differently (GitLab nests groups, `group/subgroup/project`), so each
/// gets its own type when it arrives.
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

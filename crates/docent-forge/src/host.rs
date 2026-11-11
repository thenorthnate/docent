use crate::ConnectError;

/// A forge host name, such as `github.com` or a GitHub Enterprise host.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Host(String);

impl Host {
    /// Accepts a plain DNS name only: ASCII letters, digits, `-` and `.`,
    /// with no empty labels and no label starting or ending in `-`. That rules
    /// out flags, URLs, ports and whitespace before the host reaches `gh`.
    pub fn parse(input: &str) -> Result<Self, ConnectError> {
        let host = input.trim().to_ascii_lowercase();
        let label_ok = |label: &str| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        };
        if host.len() > 253 || !host.split('.').all(label_ok) {
            return Err(ConnectError::InvalidHost(input.to_string()));
        }
        Ok(Self(host))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_normalizes_host_names() {
        assert_eq!(Host::parse("github.com").unwrap().as_str(), "github.com");
        assert_eq!(
            Host::parse("  GitHub.Example.COM ").unwrap().as_str(),
            "github.example.com"
        );
        assert_eq!(Host::parse("ghe-1.corp").unwrap().as_str(), "ghe-1.corp");
    }

    /// The host becomes an argument to `gh`, so nothing that could read as a
    /// flag, a second argument or a URL may get through.
    #[test]
    fn rejects_anything_that_is_not_a_plain_host_name() {
        for hostile in [
            "",
            "   ",
            "-h",
            "--hostname=evil.com",
            ".github.com",
            "github.com.",
            "git hub.com",
            "github.com --help",
            "https://github.com",
            "github.com/org",
            "github.com:443",
            "user@github.com",
            "github.com\n--version",
            "github..com",
            "gïthub.com",
            &"a".repeat(254),
        ] {
            assert_eq!(
                Host::parse(hostile),
                Err(ConnectError::InvalidHost(hostile.to_string())),
                "{hostile:?} should be rejected"
            );
        }
    }
}

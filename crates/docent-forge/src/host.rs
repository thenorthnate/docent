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
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        };
        if host.len() > 253 || !host.split('.').all(label_ok) {
            return Err(ConnectError::InvalidHost(input.to_string()));
        }
        Ok(Self(host))
    }

    /// The REST API base: github.com's own API host, or `/api/v3` on a
    /// GitHub Enterprise Server.
    pub fn api_base(&self) -> String {
        match self.0.as_str() {
            "github.com" => "https://api.github.com".to_string(),
            host => format!("https://{host}/api/v3"),
        }
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

    #[test]
    fn api_base_is_api_github_com_or_the_enterprise_v3_path() {
        let base = |h: &str| Host::parse(h).unwrap().api_base();
        assert_eq!(base("github.com"), "https://api.github.com");
        assert_eq!(base("ghe.corp.test"), "https://ghe.corp.test/api/v3");
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

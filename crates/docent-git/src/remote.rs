//! Which forge repo a remote URL points at.

/// A forge repo named by a remote URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteRepo {
    pub host: String,
    pub owner: String,
    pub name: String,
}

impl RemoteRepo {
    /// Reads `host`, `owner` and `name` from an `https://`, `http://`,
    /// `ssh://`, `git://` or scp-style (`git@host:owner/name.git`) URL.
    /// Local paths and anything else give `None`.
    pub fn parse(url: &str) -> Option<Self> {
        let (authority, path) = match url.split_once("://") {
            Some((scheme, rest)) => {
                if !matches!(scheme, "https" | "http" | "ssh" | "git") {
                    return None;
                }
                rest.split_once('/')?
            }
            // scp-style `[user@]host:path`. A path starting with `/` is an
            // absolute path on that host, not `owner/name`.
            None => {
                let (authority, path) = url.split_once(':')?;
                if authority.contains('/') || path.starts_with('/') {
                    return None;
                }
                (authority, path)
            }
        };
        let host = authority.rsplit('@').next()?;
        let host = host.split(':').next()?;
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let (owner, name) = path.split_once('/')?;
        if host.is_empty() || owner.is_empty() || name.is_empty() || name.contains('/') {
            return None;
        }
        Some(Self {
            host: host.to_string(),
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    /// Forge host and repo names are case-insensitive.
    pub fn is(&self, host: &str, owner: &str, name: &str) -> bool {
        self.host.eq_ignore_ascii_case(host)
            && self.owner.eq_ignore_ascii_case(owner)
            && self.name.eq_ignore_ascii_case(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(url: &str) -> Option<(String, String, String)> {
        RemoteRepo::parse(url).map(|r| (r.host, r.owner, r.name))
    }

    fn repo(host: &str, owner: &str, name: &str) -> Option<(String, String, String)> {
        Some((host.into(), owner.into(), name.into()))
    }

    #[test]
    fn reads_https_urls_with_or_without_dot_git() {
        let hello = repo("github.com", "octocat", "Hello-World");
        assert_eq!(parsed("https://github.com/octocat/Hello-World.git"), hello);
        assert_eq!(parsed("https://github.com/octocat/Hello-World"), hello);
        assert_eq!(parsed("https://github.com/octocat/Hello-World/"), hello);
        assert_eq!(parsed("http://github.com/octocat/Hello-World.git"), hello);
    }

    #[test]
    fn ignores_credentials_and_ports_in_the_authority() {
        assert_eq!(
            parsed("https://x-access-token:abc@ghe.example.com:8443/team/app.git"),
            repo("ghe.example.com", "team", "app")
        );
        assert_eq!(
            parsed("ssh://git@ghe.example.com:2222/team/app.git"),
            repo("ghe.example.com", "team", "app")
        );
    }

    #[test]
    fn reads_ssh_scp_style_and_git_protocol_urls() {
        let hello = repo("github.com", "octocat", "Hello-World");
        assert_eq!(
            parsed("ssh://git@github.com/octocat/Hello-World.git"),
            hello
        );
        assert_eq!(parsed("git@github.com:octocat/Hello-World.git"), hello);
        assert_eq!(parsed("github.com:octocat/Hello-World"), hello);
        assert_eq!(parsed("git://github.com/octocat/Hello-World.git"), hello);
    }

    #[test]
    fn local_paths_and_other_shapes_are_not_forge_repos() {
        for url in [
            "/Users/me/code/app",
            "../app.git",
            "file:///Users/me/code/app.git",
            "C:/code/app",
            "https://github.com/octocat",
            "https://github.com/a/b/c",
            "https:///octocat/app.git",
            "git@github.com:/app.git",
            "",
        ] {
            assert_eq!(parsed(url), None, "{url}");
        }
    }

    #[test]
    fn matching_ignores_case() {
        let remote = RemoteRepo::parse("git@GitHub.com:OctoCat/hello-world.git").expect("parses");

        assert!(remote.is("github.com", "octocat", "Hello-World"));
        assert!(!remote.is("github.com", "octocat", "Spoon-Knife"));
        assert!(!remote.is("ghe.example.com", "octocat", "Hello-World"));
    }
}

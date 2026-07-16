#[derive(Debug, PartialEq, Eq)]
pub struct GitUrlInfo {
    pub host: String,
    pub owner: Option<String>,
    pub repository: Option<String>,
}

pub fn parse_git_url(url: &str) -> Option<GitUrlInfo> {
    let s = url.trim();

    // Handle standard schemes
    for scheme in &[
        "https://", "http://", "ssh://", "git://", "sso://", "rpc://",
    ] {
        if let Some(stripped) = s.strip_prefix(scheme) {
            let (host_part, path) = match stripped.find('/') {
                Some(pos) => (&stripped[..pos], Some(&stripped[pos + 1..])),
                None => (stripped, None),
            };

            let host = host_part
                .split('@')
                .last()
                .unwrap_or(host_part)
                .split(':')
                .next()?;

            if host.is_empty() {
                return None;
            }

            let mut owner = None;
            let mut repository = None;

            if let Some(path) = path {
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() >= 2 {
                    owner = Some(parts[0].to_string());
                    let mut repo_name = parts[1];
                    if let Some(pos) = repo_name.find('?') {
                        repo_name = &repo_name[..pos];
                    }
                    repository = Some(
                        repo_name
                            .strip_suffix(".git")
                            .unwrap_or(repo_name)
                            .to_string(),
                    );
                } else if parts.len() == 1 && !parts[0].is_empty() {
                    let mut repo_name = parts[0];
                    if let Some(pos) = repo_name.find('?') {
                        repo_name = &repo_name[..pos];
                    }
                    repository = Some(
                        repo_name
                            .strip_suffix(".git")
                            .unwrap_or(repo_name)
                            .to_string(),
                    );
                }
            }

            return Some(GitUrlInfo {
                host: host.to_string(),
                owner,
                repository,
            });
        }
    }

    // Handle SCP-like format: [user@]host:path
    if let Some((user_host, path)) = s.split_once(':') {
        let host = user_host.split('@').last().unwrap_or(user_host);

        // If host contains a slash, it's probably not an SCP-like URL (might be a local path)
        if host.contains('/') {
            return None;
        }

        let path_clean = path.trim_start_matches('/');
        let parts: Vec<&str> = path_clean.split('/').collect();

        let mut owner = None;
        let mut repository = None;

        if parts.len() >= 2 {
            owner = Some(parts[0].to_string());
            repository = Some(
                parts[1]
                    .strip_suffix(".git")
                    .unwrap_or(parts[1])
                    .to_string(),
            );
        } else if parts.len() == 1 && !parts[0].is_empty() {
            repository = Some(
                parts[0]
                    .strip_suffix(".git")
                    .unwrap_or(parts[0])
                    .to_string(),
            );
        }

        return Some(GitUrlInfo {
            host: host.to_string(),
            owner,
            repository,
        });
    }

    None
}

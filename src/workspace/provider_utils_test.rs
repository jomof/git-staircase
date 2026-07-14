use crate::workspace::provider_utils::*;

#[test]
fn test_parse_git_url_https() {
    assert_eq!(
        parse_git_url("https://github.com/owner/repo.git"),
        Some(GitUrlInfo {
            host: "github.com".to_string(),
            owner: Some("owner".to_string()),
            repository: Some("repo".to_string()),
        })
    );
}

#[test]
fn test_parse_git_url_ssh() {
    assert_eq!(
        parse_git_url("ssh://git@github.com/owner/repo.git"),
        Some(GitUrlInfo {
            host: "github.com".to_string(),
            owner: Some("owner".to_string()),
            repository: Some("repo".to_string()),
        })
    );
}

#[test]
fn test_parse_git_url_scp() {
    assert_eq!(
        parse_git_url("git@github.com:owner/repo.git"),
        Some(GitUrlInfo {
            host: "github.com".to_string(),
            owner: Some("owner".to_string()),
            repository: Some("repo".to_string()),
        })
    );
}

#[test]
fn test_parse_git_url_with_port() {
    assert_eq!(
        parse_git_url("ssh://git@host.com:2222/owner/repo.git"),
        Some(GitUrlInfo {
            host: "host.com".to_string(),
            owner: Some("owner".to_string()),
            repository: Some("repo".to_string()),
        })
    );
}

#[test]
fn test_parse_git_url_gerrit_style() {
    assert_eq!(
        parse_git_url("https://gerrit.googlesource.com/project"),
        Some(GitUrlInfo {
            host: "gerrit.googlesource.com".to_string(),
            owner: None,
            repository: Some("project".to_string()),
        })
    );
}

#[test]
fn test_parse_git_url_no_owner() {
    assert_eq!(
        parse_git_url("https://host.com/repo.git"),
        Some(GitUrlInfo {
            host: "host.com".to_string(),
            owner: None,
            repository: Some("repo".to_string()),
        })
    );
}

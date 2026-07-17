use git_staircase::git::GitRepo;
use git_staircase::workspace::gerrit_provider::{GerritRouteOverrides, resolve_gerrit_route};
use tempfile::TempDir;

#[test]
fn test_resolve_gerrit_route_crash() {
    let tmp = TempDir::new().unwrap();
    let repo = GitRepo::new(tmp.path().to_path_buf());

    // ARRANGE: Set up overrides with a URL that doesn't have a host part
    let overrides = GerritRouteOverrides {
        server_id: Some("https://user@".to_string()),
        project: Some("project".to_string()),
        destination_branch: Some("main".to_string()),
        transport_endpoint: None,
    };

    // ACT: Call resolve_gerrit_route.
    let result = resolve_gerrit_route(&repo, None, &overrides);

    // ASSERT: Verify it returns an error instead of panicking
    assert!(result.is_err());
}

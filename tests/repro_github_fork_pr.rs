use git_staircase::GitRepo;
use git_staircase::workspace::github_provider::{GitHubStateMachine, GitHubRoute, GitHubRepoLocator};
use git_staircase::workspace::review_provider::{FakeTransport, TransportRequest};
use tempfile::TempDir;

#[test]
fn test_github_fork_pr_creation_head() {
    let repo_dir = TempDir::new().unwrap();
    let repo = GitRepo::new(repo_dir.path().to_path_buf());
    
    let transport = FakeTransport::default();
    let state_machine = GitHubStateMachine::new(transport.clone());
    
    let route = GitHubRoute {
        installation: "github.com".into(),
        base_repository: GitHubRepoLocator {
            installation: "github.com".into(),
            owner: "upstream-owner".into(),
            repository: "repo".into(),
        },
        head_repository: Some(GitHubRepoLocator {
            installation: "github.com".into(),
            owner: "fork-owner".into(),
            repository: "repo".into(),
        }),
        destination_branch: "refs/heads/main".into(),
        remote_name: "origin".into(),
    };
    
    let plan = state_machine.plan(
        &route,
        "lineage-1",
        &["abcdef0123456789abcdef0123456789abcdef01".into()],
        &["subject-1".into()],
        None,
        None,
        None,
    ).unwrap();
    
    let state = state_machine.create_state(&plan).unwrap();
    
    state_machine.publish(&repo, &plan, state, true).unwrap();
    
    let requests = transport.requests();
    let pr_request = requests.iter().find(|req| matches!(req, TransportRequest::Api { endpoint, .. } if endpoint.contains("pulls"))).expect("Should have a PR creation request");
    
    if let TransportRequest::Api { body, .. } = pr_request {
        let body_val = body.as_ref().unwrap();
        let head = body_val.get("head").unwrap().as_str().unwrap();
        
        assert!(head.contains("fork-owner:"), "GitHub PR head for forks must be 'owner:branch'. Found: {}", head);
    }
}

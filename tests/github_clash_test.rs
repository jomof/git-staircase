use git_staircase::workspace::github_provider::*;
use git_staircase::GitRepo;
use std::process::Command;
use std::fs;

fn run_git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_github_stacked_plan_branches_are_static() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init"]);
    fs::write(dir.join("a.txt"), "a").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "first"]);
    let sha1 = run_git(dir, &["rev-parse", "HEAD"]);
    
    let repo = GitRepo::new(dir.to_path_buf());
    
    let route = GitHubRoute {
        installation: "github.com".into(),
        base_repository: GitHubRepoLocator {
            installation: "github.com".into(),
            owner: "owner".into(),
            repository: "repo".into(),
        },
        head_repository: None,
        destination_branch: "refs/heads/main".into(),
        remote_name: "origin".into(),
    };
    
    // Create plan for staircase A
    let plan_a = create_github_upload_plan(&repo, &route, &vec![sha1.clone()], Some("stacked")).unwrap();
    
    // Create plan for staircase B
    let plan_b = create_github_upload_plan(&repo, &route, &vec![sha1.clone()], Some("stacked")).unwrap();
    
    // Currently they use the SAME branch name, which causes a clash
    // We WANT them to be different if they were different staircases,
    // but the API doesn't even take a staircase identifier!
    assert_ne!(plan_a.publications[0].head_branch, plan_b.publications[0].head_branch, 
        "Different staircases (implied) should have different branch names");
}

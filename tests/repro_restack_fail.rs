use git_staircase::GitRepo;
use git_staircase::core::{self, ResolvedStaircase};
use git_staircase::model::{StaircaseMetadata, Step};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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
fn test_restack_failure_leaves_repo_messy() {
    // ARRANGE
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);

    fs::write(dir.join("file.txt"), "line 1\nline 2\n").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "initial"]);
    let initial_oid = run_git(dir, &["rev-parse", "HEAD"]);

    fs::write(dir.join("file.txt"), "step1\nline 2\n").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "step1"]);
    let s1_oid = run_git(dir, &["rev-parse", "HEAD"]);
    run_git(dir, &["branch", "s1", &s1_oid]);

    run_git(dir, &["checkout", "main"]);
    run_git(dir, &["reset", "--hard", &initial_oid]);
    fs::write(dir.join("file.txt"), "conflict\nline 2\n").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "conflict"]);

    let repo = GitRepo::new(dir.to_path_buf());
    let metadata = StaircaseMetadata {
        id: "test".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            name: "s1".to_string(),
            cut: s1_oid.clone(),
            branch: Some("s1".to_string()),
        }],
        verification_policy: None,
    };
    let rs = ResolvedStaircase::Implicit(metadata);

    // ACT
    let result = core::manipulation::restack(&repo, &rs);

    // ASSERT
    if let Err(ref e) = result {
        println!("Restack failed with error: {:?}", e);
    }
    assert!(result.is_err());
    let current_branch = run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(
        current_branch, "main",
        "Repo should have been rolled back to main branch"
    );
}

use git_staircase::core;
use git_staircase::git::GitRepo;
use git_staircase::model::{ResolvedStaircase, StaircaseMetadata, Step};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_move_commits_empty_panic() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();
    run_git(repo_path, &["init", "-b", "main"]);
    fs::write(repo_path.join("file.txt"), "hello").unwrap();
    run_git(repo_path, &["add", "."]);
    run_git(repo_path, &["commit", "-m", "initial"]);
    let target_oid = Command::new("git")
        .current_dir(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let target_oid = String::from_utf8_lossy(&target_oid.stdout)
        .trim()
        .to_string();

    let repo = GitRepo::new(repo_path.to_path_buf());

    let metadata = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test-staircase".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "s1".to_string(),
                cut: target_oid.clone(),
                branch: None,
            },
            Step {
                name: "s2".to_string(),
                cut: target_oid.clone(),
                branch: None,
            },
        ],
        verification_policy: None,
    };

    let rs = ResolvedStaircase::Managed(metadata);

    // This will panic in the current implementation
    let _ = core::move_commits(&repo, &rs, 1, 0, &vec![]);
}

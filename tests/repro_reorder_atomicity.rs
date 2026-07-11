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
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()
        .unwrap();
    assert!(status.success());
}

fn run_git_out(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_reorder_partial_failure_leaves_desync() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();
    run_git(repo_path, &["init", "-b", "main"]);
    let target = {
        fs::write(repo_path.join("base.txt"), "base").unwrap();
        run_git(repo_path, &["add", "."]);
        run_git(repo_path, &["commit", "-m", "initial"]);
        run_git_out(repo_path, &["rev-parse", "HEAD"])
    };

    run_git(repo_path, &["checkout", "-b", "step1", &target]);
    let c1 = {
        fs::write(repo_path.join("f1.txt"), "1").unwrap();
        run_git(repo_path, &["add", "."]);
        run_git(repo_path, &["commit", "-m", "c1"]);
        run_git_out(repo_path, &["rev-parse", "HEAD"])
    };

    run_git(repo_path, &["checkout", "-b", "step2", &c1]);
    let c2 = {
        fs::write(repo_path.join("conflict.txt"), "2").unwrap();
        run_git(repo_path, &["add", "."]);
        run_git(repo_path, &["commit", "-m", "c2"]);
        run_git_out(repo_path, &["rev-parse", "HEAD"])
    };

    let repo = GitRepo::new(repo_path.to_path_buf());
    let metadata = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "step1".to_string(),
                cut: c1.clone(),
                branch: Some("step1".to_string()),
            },
            Step {
                name: "step2".to_string(),
                cut: c2.clone(),
                branch: Some("step2".to_string()),
            },
        ],
        verification_policy: None,
    };
    core::adopt(&repo, &metadata).unwrap();

    run_git(repo_path, &["checkout", "main"]);
    fs::write(repo_path.join("conflict.txt"), "conflict").unwrap();
    run_git(repo_path, &["add", "."]);
    run_git(repo_path, &["commit", "-m", "conflict on main"]);

    // Reorder [0, 1]. Step 1 rebases onto new main (success), Step 2 rebases onto new step 1 (fails).
    let rs = ResolvedStaircase::Managed(metadata);
    let _ = core::reorder(&repo, &rs, &[0, 1]);

    let saved_metadata = repo.read_metadata("test-id").unwrap();
    let actual_c1 = run_git_out(repo_path, &["rev-parse", "step1"]);
    // This assertion fails because metadata is not updated on reorder failure
    assert_eq!(saved_metadata.steps[0].cut, actual_c1);
}

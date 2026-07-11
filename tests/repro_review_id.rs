use git_staircase::core;
use git_staircase::GitRepo;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn run_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
    let path = dir.join(file);
    fs::write(path, contents).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

#[test]
fn test_review_id_is_empty() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    run_git(&path, &["checkout", "-b", "feature/auth-core"]);
    commit(&path, "file1.txt", "1", "commit 1\n\nChange-Id: I123");
    
    let rs = core::resolve_staircase(&repo, "feature/auth-core").unwrap().expect("Should find implicit staircase");
    let metadata = rs.metadata();
    
    let id = core::compute_identity(&repo, &metadata, git_staircase::IdentityKind::Review).unwrap();
    assert_eq!(id, "", "Expected review ID to be empty currently");
}

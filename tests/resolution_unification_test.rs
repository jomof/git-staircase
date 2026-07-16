mod common;
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
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, file: &str, content: &str, msg: &str) -> String {
    fs::write(dir.join(file), content).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

#[test]
fn test_unified_resolution_by_name() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    let root = commit(dir, "root.txt", "root", "root");
    run_git(dir, &["checkout", "-b", "base/1", &root]);
    let _c1 = commit(dir, "f1.txt", "1", "c1");
    run_git(dir, &["checkout", "-b", "base/2", "base/1"]);
    let _c2 = commit(dir, "f2.txt", "2", "c2");

    let repo = GitRepo::new(dir.to_path_buf());

    // ARRANGE: An implicit staircase exists with common prefix "base/"
    // ACT: Resolve it by tip name "base/2"
    let res = git_staircase::core::resolve_staircase(&repo, "base/2", None)
        .unwrap()
        .unwrap();

    // ASSERT: It resolves correctly (common prefix "base" is used as name)
    assert_eq!(res.staircase.metadata().name, "base");
}

#[test]
fn test_unified_resolution_by_ordinal() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    let root = commit(dir, "root.txt", "root", "root");
    run_git(dir, &["checkout", "-b", "base/1", &root]);
    let _c1 = commit(dir, "f1.txt", "1", "c1");
    run_git(dir, &["checkout", "-b", "base/2", "base/1"]);
    let _c2 = commit(dir, "f2.txt", "2", "c2");

    let repo = GitRepo::new(dir.to_path_buf());

    // ARRANGE: A 2-step implicit staircase exists
    // ACT: Resolve "base/2:1"
    let res = git_staircase::core::resolve_staircase(&repo, "base/2:1", None)
        .unwrap()
        .unwrap();

    // ASSERT: It resolves to step 0
    assert_eq!(res.step_index, Some(0));

    // ACT: Resolve "base/2:2"
    let res = git_staircase::core::resolve_staircase(&repo, "base/2:2", None)
        .unwrap()
        .unwrap();
    // ASSERT: It resolves to step 1
    assert_eq!(res.step_index, Some(1));
}

#[test]
fn test_unified_resolution_invalid_ordinal() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    let root = commit(dir, "root.txt", "root", "root");
    run_git(dir, &["checkout", "-b", "base/1", &root]);
    let _c1 = commit(dir, "f1.txt", "1", "c1");
    run_git(dir, &["checkout", "-b", "base/2", "base/1"]);
    let _c2 = commit(dir, "f2.txt", "2", "c2");

    let repo = GitRepo::new(dir.to_path_buf());

    // ACT: Resolve "base/2:0" (invalid, should be 1-based)
    let res = git_staircase::core::resolve_staircase(&repo, "base/2:0", None);
    // ASSERT: It errors
    assert!(res.is_err());

    // ACT: Resolve "base/2:3" (out of range)
    let res = git_staircase::core::resolve_staircase(&repo, "base/2:3", None);
    // ASSERT: It errors
    assert!(res.is_err());
}

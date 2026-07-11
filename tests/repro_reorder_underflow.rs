mod common;
use common::*;
use std::fs;

#[test]
fn test_reorder_underflow_error() {
    let (tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    run_git(repo_path, &["checkout", "-b", "branch-a"]);
    fs::write(repo_path.join("file_new"), "content").unwrap();
    run_git(repo_path, &["add", "file_new"]);
    run_git(repo_path, &["commit", "-m", "work"]);

    let (success, _stdout, stderr) = run_staircase(
        tmp.path(),
        &["reorder", "branch-a", "--steps", "0,1", "--onto", "main"],
    );

    assert!(!success);
    assert!(
        stderr.contains("Step indices must be 1-based"),
        "Should have returned clean error but got: {}",
        stderr
    );
}

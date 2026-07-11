mod common;
use common::*;

#[test]
fn test_list_implicit_flag_and_output() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // Create implicit staircase with 2 steps
    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // ACT: Run git staircase list --implicit
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--implicit"]);

    // ASSERT: Verify success and output format
    assert!(success, "list --implicit failed: {}", stderr);

    let output = stdout.trim();
    assert_eq!(output, "feature/auth 2 steps clean (implicit)");
}

#[test]
fn test_list_managed_output() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(dir, &["adopt", "auth", "feature/auth-core"]);
    assert!(success, "adopt failed: {}", stderr);

    // ACT: Run git staircase list --managed
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--managed"]);

    assert!(success, "list --managed failed: {}", stderr);

    let output = stdout.trim();
    // Expected: auth 1 step clean
    assert_eq!(output, "auth 1 step clean");
}

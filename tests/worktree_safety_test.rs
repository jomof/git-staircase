mod common;
use common::*;

#[test]
fn test_mutation_blocked_by_worktree_checkout() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // Create a staircase and materialize it to branches
    run_git(dir, &["checkout", "-b", "feature/a"]);
    commit(dir, "a.txt", "a", "commit a");
    let _a_oid = run_git(dir, &["rev-parse", "HEAD"]);

    run_git(dir, &["checkout", "-b", "feature/b"]);
    commit(dir, "b.txt", "b", "commit b");
    let _b_oid = run_git(dir, &["rev-parse", "HEAD"]);

    let (success, _, stderr) = run_staircase(
        dir,
        &["adopt", "my-sc", "--onto", "main", "feature/a", "feature/b"],
    );
    assert!(success, "adopt failed: {}", stderr);

    // Create a second worktree and checkout one of the staircase branches there
    let wt_path = dir.join("wt-secondary");
    run_git(
        dir,
        &["worktree", "add", wt_path.to_str().unwrap(), "feature/a"],
    );

    // Attempt to mutate the staircase (e.g., reorder)
    // This should fail because feature/a is checked out in wt-secondary
    let (success, _stdout, stderr) = run_staircase(dir, &["reorder", "my-sc", "--steps", "2,1"]);

    assert!(
        !success,
        "reorder should have failed due to worktree checkout"
    );
    assert!(
        stderr.contains("checked out in worktree"),
        "Error message should mention worktree checkout, but was: {}",
        stderr
    );
}

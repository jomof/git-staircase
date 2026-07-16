mod common;
use common::*;

#[test]
fn observation_never_adopts() {
    // ARRANGE
    // Create an unmanaged staircase (e.g., branch 'feature' ahead of 'main').
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature-1"]);
    commit(dir, "f1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "feature-2"]);
    commit(dir, "f2.txt", "2", "commit 2");

    // ACT
    // Run 'git-staircase show feature-2'.
    let (success, _stdout, stderr) = run_staircase(dir, &["show", "feature-2"]);
    assert!(success, "show command failed: {}", stderr);

    // ASSERT
    // Verify that 'git for-each-ref refs/staircases/' returns no results, proving no metadata was persisted.
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(refs.is_empty(), "Observation triggered adoption! Refs found: {}", refs);

    // ACT: Run 'git-staircase status feature-2'.
    let (success, _stdout, stderr) = run_staircase(dir, &["status", "feature-2"]);
    assert!(success, "status command failed: {}", stderr);

    // ASSERT
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(refs.is_empty(), "Status triggered adoption! Refs found: {}", refs);

    // ACT: Run 'git-staircase list --implicit'.
    let (success, _stdout, stderr) = run_staircase(dir, &["list", "--implicit"]);
    assert!(success, "list command failed: {}", stderr);

    // ASSERT
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(refs.is_empty(), "List triggered adoption! Refs found: {}", refs);
    
    // ACT: Run 'git-staircase show --ids feature-2'.
    // This is the one that is expected to FAIL and trigger adoption currently.
    let (success, _stdout, stderr) = run_staircase(dir, &["show", "feature-2", "--ids"]);
    assert!(success, "show --ids command failed: {}", stderr);

    // ASSERT
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(refs.is_empty(), "Show --ids triggered adoption! Refs found: {}", refs);
}

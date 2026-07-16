mod common;
use common::*;
use std::process::Command;

#[test]
fn observation_never_adopts() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // ARRANGE: Create a repository with a branch 'feature' that qualifies as an implicit staircase.
    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // ACT: Run 'git staircase list' and 'git staircase show feature'.
    let (success, _stdout, stderr) = run_staircase(dir, &["list", "--implicit"]);
    assert!(success, "list --implicit failed: {}", stderr);

    let (success, _stdout, stderr) = run_staircase(dir, &["show", "feature/auth"]);
    assert!(success, "show failed: {}", stderr);

    // ASSERT: Verify that no refs under 'refs/staircases/' have been created, proving the staircase remains implicit.
    let output = Command::new("git")
        .current_dir(dir)
        .args(&["for-each-ref", "refs/staircases/"])
        .output()
        .unwrap();

    let refs = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        refs.is_empty(),
        "Observation commands should not create refs under refs/staircases/. Found:\n{}",
        refs
    );

    // Verify that adopt DOES create a ref (to ensure our check is valid)
    let (success, _, stderr) = run_staircase(dir, &["adopt", "auth", "feature/auth-ui"]);
    assert!(success, "adopt failed: {}", stderr);

    let output = Command::new("git")
        .current_dir(dir)
        .args(&["for-each-ref", "refs/staircases/"])
        .output()
        .unwrap();
    let refs = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        !refs.is_empty(),
        "Adopt command SHOULD create refs under refs/staircases/"
    );
}

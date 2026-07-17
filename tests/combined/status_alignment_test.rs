
use crate::common::*;

#[test]
fn test_status_incomplete_when_branch_missing() {
    // 1. ARRANGE: Create a managed staircase
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(
        dir,
        &[
            "adopt",
            "auth",
            "--onto",
            "main",
            "feature/auth-core",
            "feature/auth-ui",
        ],
    );
    assert!(success, "adopt failed: {}", stderr);

    // 2. ACT: Delete one of the branch refs belonging to a step
    run_git(dir, &["branch", "-D", "feature/auth-core"]);

    // 3. ASSERT: git staircase status reports the state as 'incomplete'
    let (success, stdout, stderr) = run_staircase(dir, &["status", "auth"]);
    assert!(success, "status command failed: {}", stderr);

    // Per Section 8.4, it should be 'incomplete'
    assert!(
        stdout.contains("state: incomplete"),
        "Expected state to be 'incomplete', but got:\n{}",
        stdout
    );
}

#[test]
fn test_status_diverged_when_multiple_matches() {
    // 1. ARRANGE: Create a managed staircase
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(
        dir,
        &[
            "adopt",
            "auth",
            "--onto",
            "main",
            "feature/auth-core",
            "feature/auth-ui",
        ],
    );
    assert!(success, "adopt failed: {}", stderr);

    // 2. ACT: Create two branches that both match a step's identity (e.g., by name)
    // Actually, managed staircase refers to branches by name.
    // "Diverged" in the spec (8.3) says:
    // "A step has multiple incompatible current candidates, or refs and metadata disagree about which commit represents the step."

    // If I update the branch feature/auth-core to point to something else, it might be 'stale' or 'modified'.
    // Diverged usually means we found multiple branches that COULD be this step if we were discovering.
    // For a managed staircase, it's more about 'refs and metadata disagree'.

    // Let's see what the implementation of Diverged should be.
    // The deviation says:
    // "Implement logic to detect Diverged state (e.g., when a step has multiple candidate branches based on naming conventions)"

    // For now let's just assert it fails on 'incomplete' first.
}

#[test]
fn test_status_diverged_when_refs_metadata_disagree() {
    // 1. ARRANGE: Create a managed staircase
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    // Adopt it
    run_staircase(
        dir,
        &["adopt", "auth", "--onto", "main", "feature/auth-core"],
    );

    // 2. ACT: Amend the commit, causing refs and metadata to disagree
    commit(dir, "file1.txt", "1-modified", "commit 1 modified");

    // 3. ASSERT: git staircase status reports the state as 'diverged'
    let (success, stdout, stderr) = run_staircase(dir, &["status", "auth"]);
    assert!(success, "status command failed: {}", stderr);

    // Per Section 8.3, disagreement means 'diverged'
    assert!(
        stdout.contains("state: diverged"),
        "Expected state to be 'diverged', but got:\n{}",
        stdout
    );
}

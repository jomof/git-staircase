mod common;
use common::*;
use std::fs;

#[test]
fn test_reorder_non_atomic() {
    let (tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    // Create a staircase with two branches
    run_git(repo_path, &["checkout", "-b", "branch-a"]);
    fs::write(repo_path.join("file_a"), "content a").unwrap();
    run_git(repo_path, &["add", "file_a"]);
    run_git(repo_path, &["commit", "-m", "commit a"]);
    let oid_a_orig = run_git(repo_path, &["rev-parse", "HEAD"]);

    run_git(repo_path, &["checkout", "-b", "branch-b"]);
    fs::write(repo_path.join("file_b"), "content b").unwrap();
    run_git(repo_path, &["add", "file_b"]);
    run_git(repo_path, &["commit", "-m", "commit b"]);
    let oid_b_orig = run_git(repo_path, &["rev-parse", "HEAD"]);

    // Create conflict for the second step in the reordered sequence
    run_git(repo_path, &["checkout", "main"]);
    fs::write(repo_path.join("conflict.txt"), "base").unwrap();
    run_git(repo_path, &["add", "conflict.txt"]);
    run_git(repo_path, &["commit", "-m", "base conflict"]);

    run_git(repo_path, &["checkout", "branch-a"]);
    fs::write(repo_path.join("conflict.txt"), "content a").unwrap();
    run_git(repo_path, &["add", "conflict.txt"]);
    run_git(repo_path, &["commit", "-m", "commit a conflict"]);

    run_git(repo_path, &["checkout", "branch-b"]);
    fs::write(repo_path.join("conflict.txt"), "content b").unwrap();
    run_git(repo_path, &["add", "conflict.txt"]);
    run_git(repo_path, &["commit", "-m", "commit b conflict"]);

    // Attempt to reorder [branch-b, branch-a] onto main
    // Rebase of branch-b succeeds, rebase of branch-a fails.
    let (success, _stdout, stderr) = run_staircase(
        tmp.path(),
        &["reorder", "branch-b", "--order", "2,1", "--onto", "main"],
    );

    assert!(!success, "Reorder should have failed. Stderr: {}", stderr);

    let oid_b_new = run_git(repo_path, &["rev-parse", "branch-b"]);

    // Branch B was moved (rebased successfully)
    assert_ne!(
        oid_b_new, oid_b_orig,
        "branch-b should have been rebased even though the command failed"
    );

    // The bug is that the command didn't roll back the changes to branch-b,
    // leaving the repository in a partially reordered state that is inconsistent with the metadata.
}

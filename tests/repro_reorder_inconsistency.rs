use git_staircase::core::persistence;
mod common;
use common::*;
use git_staircase::core::{adopt, reorder, resolve_staircase};
use std::fs;

#[test]
fn test_reorder_metadata_inconsistency() {
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    // 1. Create a staircase: A -> B -> C
    run_git(repo_path, &["checkout", "-b", "branch-a"]);
    commit(repo_path, "a.txt", "a", "a");
    let oid_a = run_git(repo_path, &["rev-parse", "HEAD"]);

    run_git(repo_path, &["checkout", "-b", "branch-b"]);
    commit(repo_path, "b.txt", "b", "b");
    let oid_b_orig = run_git(repo_path, &["rev-parse", "HEAD"]);

    run_git(repo_path, &["checkout", "-b", "branch-c"]);
    commit(repo_path, "c.txt", "c", "c");
    let oid_c_orig = run_git(repo_path, &["rev-parse", "HEAD"]);

    // 2. Resolve and Adopt it to make it managed
    let rs = resolve_staircase(&repo, "branch-c", Some("main"))
        .unwrap()
        .unwrap();
    let adopted_meta = adopt(&repo, rs.metadata()).unwrap();
    let id = adopted_meta.id.clone();

    // Resolve by ID to get the Managed variant
    let rs = resolve_staircase(&repo, &id, Some("main"))
        .unwrap()
        .unwrap();
    assert!(rs.is_managed());

    // 3. Create a conflict for reordering
    run_git(repo_path, &["checkout", "branch-c"]);
    fs::write(repo_path.join("conflict.txt"), "content c").unwrap();
    run_git(repo_path, &["add", "conflict.txt"]);
    run_git(repo_path, &["commit", "-m", "conflict c"]);
    let oid_c_before = run_git(repo_path, &["rev-parse", "HEAD"]);

    run_git(repo_path, &["checkout", "branch-b"]);
    fs::write(repo_path.join("conflict.txt"), "content b").unwrap();
    run_git(repo_path, &["add", "conflict.txt"]);
    run_git(repo_path, &["commit", "-m", "conflict b"]);
    let oid_b_before = run_git(repo_path, &["rev-parse", "HEAD"]);

    // Attempt reorder to: [branch-a, branch-c, branch-b]
    // Rebase of branch-c onto branch-a (step 1) will succeed.
    // Rebase of branch-b onto branch-c' (step 2) will fail.
    let result = reorder(&repo, &rs, &[0, 2, 1], git_staircase::core::ReorderOptions { no_restack: false });
    assert!(result.is_err(), "Reorder should fail due to conflict");

    // 4. Verify metadata remains UNCHANGED on failure
    let staircases = persistence::list_staircases(&repo).unwrap();
    assert!(!staircases.is_empty());
    let meta = staircases
        .iter()
        .find(|s| s.id == id)
        .expect("Managed staircase not found");

    // Metadata should still point to ORIGINAL OIDs (from before conflicts and before reorder attempt)
    // Actually, it should point to what was in the metadata when reorder was called.
    // When we adopted, it had oid_a, oid_b_orig, oid_c_orig.
    assert_eq!(meta.steps[0].cut, oid_a, "Step 0 cut should be unchanged");
    assert_eq!(
        meta.steps[1].cut, oid_b_orig,
        "Step 1 cut should be unchanged"
    );
    assert_eq!(
        meta.steps[2].cut, oid_c_orig,
        "Step 2 cut should be unchanged"
    );

    // 5. Verify branches were ROLLED BACK to their state just before reorder started
    let current_oid_b = run_git(repo_path, &["rev-parse", "branch-b"]);
    assert_eq!(
        current_oid_b, oid_b_before,
        "branch-b should be rolled back to its state before reorder started"
    );

    let current_oid_c = run_git(repo_path, &["rev-parse", "branch-c"]);
    assert_eq!(
        current_oid_c, oid_c_before,
        "branch-c should be rolled back to its state before reorder started"
    );
}

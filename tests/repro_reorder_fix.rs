mod common;
use common::*;

#[test]
fn test_reorder_rollback_fails() {
    let (tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    run_git(repo_path, &["checkout", "-b", "a"]);
    commit(repo_path, "conflict.txt", "content a", "a");
    let _oid_a = run_git(repo_path, &["rev-parse", "HEAD"]);

    run_git(repo_path, &["checkout", "-b", "b"]);
    commit(repo_path, "b.txt", "content b", "b");
    let oid_b = run_git(repo_path, &["rev-parse", "HEAD"]);

    // Force a conflict on 'a'
    run_git(repo_path, &["checkout", "main"]);
    commit(repo_path, "conflict.txt", "content main", "conflict");
    
    // Reorder [b, a] onto main. 
    // current_base starts at main.
    // Step 0 is 'b' (old_idx=1). old_parent_oid = old_steps[0].cut (oid_a).
    // Rebase b onto main from a. (This moves only commit b).
    // Step 1 is 'a' (old_idx=0). old_parent_oid = merge_base(a, main).
    // Rebase a onto new_b from old_parent_oid.
    // Conflict!
    
    let (success, _, _) = run_staircase(tmp.path(), &["reorder", "b", "--order", "2,1", "--onto", "main"]);
    assert!(!success, "Reorder should have failed");
    
    let oid_b_after = run_git(repo_path, &["rev-parse", "b"]);
    assert_eq!(oid_b, oid_b_after, "Branch b should have been rolled back!");
}

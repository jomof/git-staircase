use git_staircase::core::*;
use git_staircase::model::*;
use git_staircase::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

mod common;
use common::*;

// --- repro_reorder_atomicity.rs ---

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

// --- repro_reorder_fix.rs ---

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

    let (success, _, _) = run_staircase(
        tmp.path(),
        &["reorder", "b", "--order", "2,1", "--onto", "main"],
    );
    assert!(!success, "Reorder should have failed");

    let oid_b_after = run_git(repo_path, &["rev-parse", "b"]);
    assert_eq!(oid_b, oid_b_after, "Branch b should have been rolled back!");
}

// --- repro_reorder_inconsistency.rs ---

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
    let result = reorder(
        &repo,
        &rs,
        &[0, 2, 1],
        git_staircase::core::ReorderOptions { no_restack: false },
    );
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

// --- repro_reorder_underflow.rs ---

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
        &["reorder", "branch-a", "--order", "0,1", "--onto", "main"],
    );

    assert!(!success);
    assert!(
        stderr.contains("Step indices must be 1-based"),
        "Should have returned clean error but got: {}",
        stderr
    );
}

// --- repro_restack.rs ---

#[test]
fn test_restack_propagation() {
    // ARRANGE
    let ctx = TestContext::new();

    // 1. Repo already initialized with one commit by TestContext::new() (via setup_repo)

    // 2. Create a chain of 3 branches: s1 -> s2 -> s3
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("s1.txt", "s1", "s1 commit");

    ctx.run_git(&["checkout", "-b", "s2"]);
    let c2 = ctx.commit("s2.txt", "s2", "s2 commit");

    ctx.run_git(&["checkout", "-b", "s3"]);
    let c3 = ctx.commit("s3.txt", "s3", "s3 commit");

    // 3. Adopt as a staircase
    let sc = StaircaseMetadata {
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
            Step {
                id: String::new(),
                name: "s3".to_string(),
                cut: c3.clone(),
                branch: Some("s3".to_string()),
            },
        ],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();

    // 4. Modify s1 by rebasing it onto a new commit on main
    ctx.run_git(&["checkout", "main"]);
    ctx.commit("main2.txt", "main2", "main update");

    ctx.run_git(&["checkout", "s1"]);
    ctx.run_git(&["rebase", "main"]);
    let s1_new = ctx.run_git(&["rev-parse", "HEAD"]);

    let rs = git_staircase::core::resolve_staircase(&ctx.repo, "test", None)
        .unwrap()
        .unwrap();

    // ACT: Run restack
    git_staircase::core::restack(
        &ctx.repo,
        &rs,
        core::RebaseOptions {
            leave_upper_steps_stale: false,
        },
    )
    .unwrap();

    // ASSERT: Verify results
    let s1_final = ctx.run_git(&["rev-parse", "s1"]);
    let s2_final = ctx.run_git(&["rev-parse", "s2"]);
    let s3_final = ctx.run_git(&["rev-parse", "s3"]);

    assert_eq!(s1_final, s1_new);

    // s2 should have been rebased onto s1_new
    assert!(ctx.repo.is_ancestor(&s1_final, &s2_final).unwrap());

    // s3 SHOULD have been rebased onto s2_new
    assert!(
        ctx.repo.is_ancestor(&s2_final, &s3_final).unwrap(),
        "s3 was not rebased onto s2"
    );
}

// --- repro_restack_fail.rs ---

#[test]
fn test_restack_conflict_handling() {
    // ARRANGE
    let ctx = TestContext::new();

    // 1. Create a chain: s1
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("conflict.txt", "s1", "s1 commit");

    // 2. Adopt as a staircase
    let sc = StaircaseMetadata {
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "s1".to_string(),
            cut: c1.clone(),
            branch: Some("s1".to_string()),
        }],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();

    // 3. Modify main to conflict with s1
    ctx.run_git(&["checkout", "main"]);
    ctx.commit("conflict.txt", "main", "main update");

    // 4. Resolve and try to restack
    let rs = git_staircase::core::resolve_staircase(&ctx.repo, "test", None)
        .unwrap()
        .unwrap();

    // ACT
    let result = git_staircase::core::restack(
        &ctx.repo,
        &rs,
        core::RebaseOptions {
            leave_upper_steps_stale: false,
        },
    );

    // ASSERT
    assert!(
        result.is_err(),
        "Restack should fail when there is a conflict"
    );
}

// --- repro_restack_inconsistency.rs ---

#[test]
fn test_restack_inconsistency_on_failure() {
    let ctx = TestContext::new();

    // 1. Create a chain: main -> s1 -> s2
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("s1.txt", "s1", "s1 commit");
    ctx.run_git(&["checkout", "-b", "s2"]);
    let c2 = ctx.commit("s2.txt", "s2", "s2 commit");

    // 2. Adopt as a managed staircase
    let sc = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: "s1-id".to_string(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                id: "s2-id".to_string(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();

    // 3. Update main with a commit that conflicts with s2 but NOT s1
    ctx.run_git(&["checkout", "main"]);
    ctx.commit("main-unrelated.txt", "main", "unrelated update");
    ctx.commit("s2.txt", "conflict", "conflict with s2");

    let rs = git_staircase::core::resolve_staircase(&ctx.repo, "test", None)
        .unwrap()
        .unwrap();

    // 4. Run restack. It should succeed on s1 and fail on s2.
    let result = git_staircase::core::manipulation::restack(
        &ctx.repo,
        &rs,
        git_staircase::core::RebaseOptions {
            leave_upper_steps_stale: false,
        },
    );
    assert!(result.is_err(), "Restack should fail on s2 conflict");

    // 5. Check consistency
    let s1_branch_oid = ctx.repo.resolve_commit("s1").unwrap();
    let meta = persistence::read_metadata(&ctx.repo, "test-id").unwrap();
    let s1_meta_oid = meta.steps[0].cut.clone();

    // FAILURE: s1_branch_oid is c1 (rolled back), but s1_meta_oid is the new rebased OID
    assert_eq!(
        s1_branch_oid, s1_meta_oid,
        "Branch and metadata should be consistent after failed restack"
    );
}

// --- repro_move_crash.rs ---

#[test]
fn test_move_commits_empty_panic() -> anyhow::Result<()> {
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;
    let target_oid = run_git(repo_path, &["rev-parse", "HEAD"]);

    let metadata = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test-staircase".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: target_oid.clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: target_oid.clone(),
                branch: None,
            },
        ],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };

    let rs = ResolvedStaircase::Managed(metadata);

    let _ = core::move_commits(&repo, &rs, 1, 0, &[]);
    Ok(())
}

// --- repro_move_empty_v2.rs ---

#[test]
fn test_move_commit_creates_empty_step_violating_invariant() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let c1 = ctx.commit("f1.txt", "1", "c1");
    let c2 = ctx.commit("f2.txt", "2", "c2");

    let meta = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: target,
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };
    let rs = ResolvedStaircase::Managed(meta.clone());
    persistence::write_metadata(&ctx.repo, &meta).unwrap();

    // ACT: Move the only commit of s2 into s1
    manipulation::move_commits(&ctx.repo, &rs, 1, 0, &[c2.clone()]).unwrap();

    // ASSERT: The resulting staircase is now considered invalid by the core logic
    let updated_meta = persistence::read_metadata(&ctx.repo, "test-id").unwrap();
    let result = adopt(&ctx.repo, &updated_meta);
    assert!(
        result.is_err(),
        "Staircase with empty step should be rejected by validation logic"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("every step must be non-empty")
    );
}

// --- repro_pipe_branch.rs ---

#[test]
fn test_local_branches_with_pipe() -> anyhow::Result<()> {
    // ARRANGE
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    // Create a branch with a pipe in its name
    run_git(repo_path, &["checkout", "-b", "feat|pipe"]);

    // ACT
    let branches = repo.local_branches(None)?;

    // ASSERT
    let pipe_branch = branches.iter().find(|b| b.refname.contains("feat|pipe"));
    assert!(
        pipe_branch.is_some(),
        "Branch 'feat|pipe' should be found correctly. Found: {:?}",
        branches
    );

    Ok(())
}

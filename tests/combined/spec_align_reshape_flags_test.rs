
use crate::common::*;
use git_staircase::core::persistence;

#[test]
fn test_rebase_leave_upper_steps_stale() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "s1"]);
    let _c1 = ctx.commit("s1.txt", "s1", "s1 commit");
    ctx.run_git(&["checkout", "-b", "s2"]);
    let c2 = ctx.commit("s2.txt", "s2", "s2 commit");

    let (success, _, stderr) = ctx.run_staircase(&["adopt", "test", "s1", "s2"]);
    assert!(success, "adopt failed: {}", stderr);

    // Update main
    ctx.run_git(&["checkout", "main"]);
    let main_new = ctx.commit("main.txt", "new", "main update");

    // Rebase s1 onto main, leaving s2 stale
    let (success, _, stderr) = ctx.run_staircase(&[
        "rebase",
        "test:1",
        "--onto",
        "main",
        "--leave-upper-steps-stale",
    ]);
    assert!(success, "rebase failed: {}", stderr);

    // Verify s1 is rebased onto main_new
    let s1_parent = ctx.run_git(&["rev-parse", "s1~1"]);
    assert_eq!(s1_parent, main_new);

    // Verify s2 is NOT rebased
    let s2_oid = ctx.run_git(&["rev-parse", "s2"]);
    assert_eq!(s2_oid, c2);

    // Verify status shows state: stale
    let (success, stdout, stderr) = ctx.run_staircase(&["status", "test"]);
    assert!(success, "status failed: {}", stderr);
    assert!(
        stdout.contains("state: stale"),
        "Expected state: stale, but output was:\n{}",
        stdout
    );
}

#[test]
fn test_reorder_restacks_by_default() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("s1.txt", "s1", "s1 commit");
    ctx.run_git(&["checkout", "-b", "s2"]);
    let c2 = ctx.commit("s2.txt", "s2", "s2 commit");

    let (success, _, stderr) = ctx.run_staircase(&["adopt", "test", "s1", "s2"]);
    assert!(success, "adopt failed: {}", stderr);

    // Reorder s2 before s1 using the canonical complete permutation.
    let (success, _, stderr) = ctx.run_staircase(&["reorder", "test", "--steps", "2,1"]);
    assert!(success, "reorder failed: {}", stderr);

    // Reordering rewrites both conceptual steps while preserving their identities.
    let s1_oid = ctx.run_git(&["rev-parse", "s1"]);
    assert_ne!(s1_oid, c1);
    let s2_oid = ctx.run_git(&["rev-parse", "s2"]);
    assert_ne!(s2_oid, c2);

    // Verify the fully restacked result is clean.
    let (success, stdout, stderr) = ctx.run_staircase(&["status", "test"]);
    assert!(success, "status failed: {}", stderr);
    assert!(
        stdout.contains("state: clean"),
        "Expected state: clean, but output was:\n{}",
        stdout
    );
}

#[test]
fn test_split_no_ref_triggers_adoption() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("1.txt", "1", "c1");
    let _c2 = ctx.commit("2.txt", "2", "c2");

    // We have an implicit staircase (s1 branch has 2 commits)
    // Split it at c1 with --no-ref.
    let (success, _stdout, stderr) = ctx.run_staircase(&[
        "split", "s1:1", "--at", &c1, "--branch", "s1-part1", "--no-ref",
    ]);
    assert!(success, "split failed: {}", stderr);

    // Verify it was adopted. The name should be "s1"
    let (success, stdout, stderr) = ctx.run_staircase(&["list", "--porcelain"]);
    assert!(success, "list failed: {}", stderr);
    assert!(
        stdout.contains("s1"),
        "Staircase 's1' should be adopted and listed, but output was:\n{}",
        stdout
    );

    // Also check that it has 2 steps in metadata
    let meta = persistence::read_metadata(&ctx.repo, "s1").expect("Should find adopted metadata");
    assert_eq!(meta.steps.len(), 2);
    assert_eq!(meta.steps[0].name, "s1-part1");
    assert_eq!(meta.steps[0].branch, None);
    assert_eq!(meta.steps[1].name, "s1");
    assert_eq!(meta.steps[1].branch, Some("s1".to_string()));
}

#[test]
fn test_join_keep_boundary_ref_triggers_adoption() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "s1"]);
    let _c1 = ctx.commit("s1.txt", "s1", "s1 commit");
    ctx.run_git(&["checkout", "-b", "s2"]);
    let _c2 = ctx.commit("s2.txt", "s2", "s2 commit");

    // We have implicit staircase.
    // Join s1 and s2, keeping s1 branch ref.
    let (success, _, stderr) = ctx.run_staircase(&["join", "s:1", "2", "--keep-boundary-ref"]);
    assert!(success, "join failed: {}", stderr);

    // Verify it was adopted
    let (success, stdout, stderr) = ctx.run_staircase(&["list", "--porcelain"]);
    assert!(success, "list failed: {}", stderr);
    assert!(
        stdout.contains("s"),
        "Staircase 's' should be adopted, but output was:\n{}",
        stdout
    );

    // Verify s1 branch still exists
    let s1_oid = ctx.run_git(&["rev-parse", "--verify", "s1"]);
    assert!(!s1_oid.is_empty());
}

#[test]
fn test_drop_leave_descendants_stale() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "s1"]);
    let _c1 = ctx.commit("s1.txt", "s1", "s1 commit");
    ctx.run_git(&["checkout", "-b", "s2"]);
    let _c2 = ctx.commit("s2.txt", "s2", "s2 commit");
    ctx.run_git(&["checkout", "-b", "s3"]);
    let c3 = ctx.commit("s3.txt", "s3", "s3 commit");

    let (success, _, stderr) = ctx.run_staircase(&["adopt", "test", "s1", "s2", "s3"]);
    assert!(success, "adopt failed: {}", stderr);

    // Drop s2, leaving s3 stale (not restacked)
    let (success, _, stderr) = ctx.run_staircase(&["drop", "test:2", "--leave-descendants-stale"]);
    assert!(success, "drop failed: {}", stderr);

    // Verify s3 branch is NOT rewritten (still points to c3)
    let s3_oid = ctx.run_git(&["rev-parse", "s3"]);
    assert_eq!(s3_oid, c3);
}

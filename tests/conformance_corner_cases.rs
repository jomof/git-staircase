mod common;

use common::TestContext;

#[test]
fn empty_step_is_rejected_before_mutation() {
    // ARRANGE: A repository with a one-commit staircase.
    let ctx = TestContext::new();
    let base = ctx.run_git(&["rev-parse", "HEAD"]);

    ctx.run_git(&["checkout", "-b", "feature"]);
    let c1 = ctx.commit("f1.txt", "c1", "c1");
    ctx.run_git(&["checkout", "main"]);

    // ACT: Attempt to split that step into two at the boundary (c1 or base).

    // Split at base
    let (success, _stdout, stderr) = ctx.run_staircase(&[
        "split",
        "feature:1",
        "--at",
        &base,
        "--branch",
        "new-step",
        "--onto",
        "main",
    ]);

    // ASSERT: The command fails with a diagnostic error.
    assert!(!success, "Split at base should fail");
    assert!(
        stderr.contains("Cannot split at step boundaries")
            || stderr.contains("every step must be non-empty"),
        "Error message should indicate non-empty step requirement. Stderr: {}",
        stderr
    );

    // Split at c1
    let (success, _stdout, stderr) = ctx.run_staircase(&[
        "split",
        "feature:1",
        "--at",
        &c1,
        "--branch",
        "new-step",
        "--onto",
        "main",
    ]);
    assert!(!success, "Split at c1 should fail");
    assert!(
        stderr.contains("Cannot split at step boundaries")
            || stderr.contains("every step must be non-empty"),
        "Error message should indicate non-empty step requirement. Stderr: {}",
        stderr
    );
}

#[test]
fn rebase_onto_cut_is_rejected() {
    // ARRANGE: base -> c1 -> c2
    let ctx = TestContext::new();
    let base = ctx.run_git(&["rev-parse", "HEAD"]);

    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("f1.txt", "c1", "c1");
    ctx.run_git(&["checkout", "-b", "s2"]);
    let _c2 = ctx.commit("f2.txt", "c2", "c2");

    let (success, _, stderr) = ctx.run_staircase(&["adopt", "test", "s1", "s2", "--onto", &base]);
    assert!(success, "adopt failed: {}", stderr);

    // ACT: Rebase onto c1. Step 1 (s1) becomes empty.
    let (success, _stdout, stderr) = ctx.run_staircase(&["rebase", "test", "--onto", &c1]);

    // ASSERT: Fails with diagnostic error.
    assert!(!success, "Rebase onto cut should fail");
    assert!(
        stderr.contains("every step must be non-empty"),
        "Error should mention empty step. Stderr: {}",
        stderr
    );
}

#[test]
fn explicit_staircase_with_empty_step_is_rejected() {
    // ARRANGE: base -> c1
    let ctx = TestContext::new();
    let _base = ctx.run_git(&["rev-parse", "HEAD"]);
    let c1 = ctx.commit("f1.txt", "c1", "c1");

    ctx.run_git(&["branch", "s1", &c1]);

    // ACT: Try to use --steps s1,s1. Step 2 (s1..s1) is empty.
    let (success, _stdout, stderr) = ctx.run_staircase(&["status", "--steps", "s1,s1"]);

    // ASSERT: Fails with diagnostic error.
    assert!(!success, "Explicit staircase with empty step should fail");
    assert!(
        stderr.contains("every step must be non-empty"),
        "Error should mention empty step. Stderr: {}",
        stderr
    );
}

#[test]
fn main_ahead_of_anchor_is_valid_work() {
    // ARRANGE: Repository with 'main' ahead of its base.
    let ctx = TestContext::new();
    let base = ctx.run_git(&["rev-parse", "HEAD"]);
    ctx.commit("work.txt", "some work", "commit on main");

    // ACT: Run 'git-staircase list'.
    let (success, stdout, stderr) = ctx.run_staircase(&["list", "--onto", &base]);

    // ASSERT: 'main' is present in output with '(implicit)' status.
    assert!(success, "list failed: {}", stderr);
    assert!(
        stdout.contains("main"),
        "main should be listed. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("(implicit)"),
        "main should be implicit. Output: {}",
        stdout
    );
}

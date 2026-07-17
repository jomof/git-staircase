
use crate::common::*;

#[test]
fn test_land_stepwise_integration() {
    // ARRANGE: Managed staircase with stepwise policy.
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "step1"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "step2"]);
    let c2 = ctx.commit("file2.txt", "2", "commit 2");

    ctx.run_git(&["checkout", "main"]);

    // Adopt it with stepwise policy
    let (ok, stdout, stderr) = ctx.run_staircase(&[
        "adopt",
        "auth",
        "--onto",
        "main",
        "--landing-policy",
        "stepwise",
        "step1",
        "step2",
    ]);
    assert!(ok, "Adopt failed: {} {}", stdout, stderr);

    // ACT: 'git staircase land'.
    let (ok, stdout, stderr) = ctx.run_staircase(&["land", "auth"]);

    // ASSERT
    assert!(ok, "Land failed: {} {}", stdout, stderr);

    // Verify main branch has both commits and they are integrated.
    let main_oid = ctx.run_git(&["rev-parse", "main"]);

    // Check if main is now at c2 (because of --ff-only)
    assert_eq!(
        main_oid, c2,
        "main should be at the tip of the staircase after stepwise land"
    );

    let history = ctx.run_git(&["log", "--oneline", "main"]);
    assert!(history.contains("commit 1"));
    assert!(history.contains("commit 2"));
}

#[test]
fn test_land_aggregate_integration() {
    // ARRANGE: Managed staircase with aggregate-only policy.
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "step1"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "step2"]);
    let c2 = ctx.commit("file2.txt", "2", "commit 2");

    ctx.run_git(&["checkout", "main"]);

    // Adopt it with aggregate-only policy
    let (ok, stdout, stderr) = ctx.run_staircase(&[
        "adopt",
        "auth",
        "--onto",
        "main",
        "--landing-policy",
        "aggregate-only",
        "step1",
        "step2",
    ]);
    assert!(ok, "Adopt failed: {} {}", stdout, stderr);

    // ACT: 'git staircase land'.
    let (ok, stdout, stderr) = ctx.run_staircase(&["land", "auth"]);

    // ASSERT
    assert!(ok, "Land failed: {} {}", stdout, stderr);

    // Verify main branch has both commits and they are integrated.
    let main_oid = ctx.run_git(&["rev-parse", "main"]);
    assert_eq!(main_oid, c2);
}

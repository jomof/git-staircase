mod common;
use common::TestContext;

#[test]
fn test_reorder_no_restack_validation_bypass() {
    let ctx = TestContext::new();

    // ARRANGE
    ctx.run_git(&["checkout", "-b", "branch1"]);
    ctx.commit("f1", "c1", "commit 1");
    ctx.run_git(&["checkout", "-b", "branch2"]);
    ctx.commit("f2", "c2", "commit 2");

    // Create a staircase with two steps.
    let (success, _, _) = ctx.run_staircase(&[
        "adopt",
        "my-staircase",
        "--onto",
        "main",
        "branch1",
        "branch2",
    ]);
    assert!(success);

    // ACT
    // Reorder steps to [2, 1]. This SHOULD be invalid because branch2 descends from branch1.
    let (success, stdout, stderr) =
        ctx.run_staircase(&["reorder", "my-staircase", "--steps", "2,1"]);

    // ASSERT
    // It succeeded! (This is the bug: validation bypass)
    assert!(
        !success,
        "Reorder should have failed due to invalid ancestry: {} {}",
        stdout, stderr
    );

    // Now verify the state is broken.
    // Status should likely report an error or at least show it's not clean.
    let (_success, stdout, stderr) = ctx.run_staircase(&["status", "my-staircase"]);
    // Since the structure is broken, status might fail.
    println!("Status stdout: {}", stdout);
    println!("Status stderr: {}", stderr);

    // If it's broken, it might return success but show stale steps.
    // But actually, it should have been blocked at the reorder step.
}

mod common;
use common::TestContext;
use std::fs;

#[test]
fn test_restack_manual_strategy_silent_conflict() {
    let ctx = TestContext::new();

    // 1. Create a staircase with two steps
    ctx.run_git(&["checkout", "-b", "s1"]);
    ctx.commit("conflict.txt", "line 1\n", "step 1");
    ctx.run_git(&["checkout", "-b", "s2"]);
    ctx.commit("conflict.txt", "line 1\nline 2 from s2\n", "step 2");

    // Adopt it
    let (success, _, _) = ctx.run_staircase(&["adopt", "my-staircase", "s1", "s2"]);
    assert!(success);

    // 2. Materialize a conflicting change into Step 1 (s1)
    ctx.run_git(&["checkout", "s1"]);
    fs::write(ctx.path().join("conflict.txt"), "line 1\nline 2 from s1\n").unwrap();
    ctx.run_staircase(&["draft", "attach", "my-staircase", "--step", "s1"]);

    // 3. Materialize! This will restack s2 onto the new s1 using Manual strategy.
    // s2 changed line 2 to "line 2 from s2".
    // s1 changed line 2 to "line 2 from s1".
    // This is a conflict!
    let (success, _, stderr) = ctx.run_staircase(&["draft", "materialize", "--all-tracked"]);

    // The fix is that 'success' should be FALSE.
    assert!(!success, "Expected materialize to fail due to conflicts!");
    assert!(
        stderr.contains("merge-tree failed (likely due to conflicts)"),
        "Unexpected error message: {}",
        stderr
    );
}

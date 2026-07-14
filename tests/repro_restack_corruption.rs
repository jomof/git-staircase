mod common;
use common::TestContext;
use std::fs;

#[test]
fn test_restack_manual_strategy_corrupts_data() {
    let ctx = TestContext::new();

    // 1. Create a staircase with two steps
    ctx.run_git(&["checkout", "-b", "s1"]);
    ctx.commit("a.txt", "1", "step 1");
    ctx.run_git(&["checkout", "-b", "s2"]);
    ctx.commit("b.txt", "2", "step 2");

    // Adopt it
    let (success, _, _) = ctx.run_staircase(&["adopt", "my-staircase", "s1", "s2"]);
    assert!(success);

    // 2. Materialize a change into Step 1 (s1)
    ctx.run_git(&["checkout", "s1"]);
    fs::write(ctx.path().join("a.txt"), "1 updated").unwrap();
    ctx.run_staircase(&["draft", "attach", "my-staircase", "--step", "s1"]);

    // 3. Materialize! This will restack s2 onto the new s1 using Manual strategy.
    let (success, _, _) = ctx.run_staircase(&["draft", "materialize", "--all-tracked"]);
    assert!(success);

    // 4. Verify Step s2's content
    let content_a = ctx.run_git(&["show", "s2:a.txt"]);

    if content_a == "1" {
        panic!(
            "BUG REPRODUCED: Data corruption! 'a.txt' was reverted to '1' during restack because Manual strategy used old tree directly."
        );
    }
}

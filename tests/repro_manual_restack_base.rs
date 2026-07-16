mod common;
use common::TestContext;
use std::fs;

#[test]
fn test_manual_restack_uses_wrong_merge_base() {
    let ctx = TestContext::new();

    // 1. Create a staircase with two steps
    ctx.run_git(&["checkout", "-b", "s1"]);
    ctx.commit("file1.txt", "1", "step 1");

    ctx.run_git(&["checkout", "-b", "s2"]);
    // Step 2, Commit A: modify file2.txt
    ctx.commit("file2.txt", "A", "step 2 - commit A");
    // Step 2, Commit B: modify file2.txt again
    ctx.commit("file2.txt", "B", "step 2 - commit B");

    // Adopt it
    let (success, _, _) = ctx.run_staircase(&["adopt", "my-staircase", "s1", "s2"]);
    assert!(success);

    // 2. Modify Step 1 to trigger restack of Step 2
    ctx.run_git(&["checkout", "s1"]);
    fs::write(ctx.path().join("file3.txt"), "new").unwrap();
    ctx.run_git(&["add", "file3.txt"]);
    ctx.run_staircase(&["draft", "attach", "my-staircase", "--step", "s1"]);

    // 3. Materialize. This uses Manual strategy for restacking Step 2.
    // If it uses s1_parent as merge base for BOTH commits in Step 2:
    // - Merge 1: base=s1_parent, s1_new, s2_commit_A. (OK, but weird).
    // - Merge 2: base=s1_parent, merge1_result, s2_commit_B.
    // s2_commit_B contains changes from s2_commit_A.
    // merge1_result also contains changes from s2_commit_A.
    // Conflict!
    let (success, _, stderr) = ctx.run_staircase(&["draft", "materialize", "--all-tracked"]);
    assert!(success, "Materialize failed: {}", stderr);

    // 4. Verify Step s2's content
    let content = ctx.run_git(&["show", "s2:file2.txt"]);
    println!("Content of file2.txt in s2: {}", content);

    if content.contains("<<<<<<<") {
        panic!(
            "BUG REPRODUCED: Data corruption! 'file2.txt' contains conflict markers because Manual strategy used wrong merge base."
        );
    }

    if content.trim() != "B" {
        panic!(
            "BUG REPRODUCED: Data corruption! 'file2.txt' content is '{}', expected 'B'.",
            content
        );
    }
}

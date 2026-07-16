mod common;
use common::TestContext;
use std::fs;

#[test]
fn test_materialize_corrupts_multi_commit_step() {
    let ctx = TestContext::new();

    // 1. Create a staircase with two steps
    // Step 1: 1 commit
    ctx.run_git(&["checkout", "-b", "s1"]);
    ctx.commit("file1.txt", "1", "step 1 commit");

    // Step 2: 2 commits
    ctx.run_git(&["checkout", "-b", "s2"]);
    ctx.commit("file2.txt", "2a", "step 2 commit A");
    ctx.commit("file2.txt", "2a\n2b", "step 2 commit B");

    // Adopt it
    let (success, _, _) = ctx.run_staircase(&["adopt", "my-staircase", "s1", "s2"]);
    assert!(success);

    // 2. Create a draft for Step 1
    ctx.run_git(&["checkout", "s1"]);
    fs::write(ctx.path().join("file1.txt"), "1 updated").unwrap();

    // Attach draft to s1
    let (success, _, _) = ctx.run_staircase(&["draft", "attach", "my-staircase", "--step", "s1"]);
    assert!(success);

    // 3. Materialize. This uses RestackStrategy::Manual for s2.
    let (success, stdout, stderr) = ctx.run_staircase(&["draft", "materialize", "--all-tracked"]);

    if !success {
        if stderr.contains("merge-tree failed") || stdout.contains("merge-tree failed") {
            panic!(
                "BUG REPRODUCED: Materialize failed with conflict because of incorrect merge base in Manual restack."
            );
        }
        panic!("Materialize failed unexpectedly: {}\n{}", stdout, stderr);
    }

    // 4. Verify Step 2 content
    let file2_content = ctx.run_git(&["show", "s2:file2.txt"]);
    if file2_content.trim() != "2a\n2b" {
        panic!(
            "BUG REPRODUCED: Step 2 content is incorrect after materialize. Expected '2a\\n2b', got '{}'",
            file2_content.trim()
        );
    }
}

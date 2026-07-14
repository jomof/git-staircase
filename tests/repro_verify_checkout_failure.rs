mod common;
use common::TestContext;

#[test]
fn test_verify_fails_to_restore_branch_if_build_leaves_dirty_worktree() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "base-branch"]);
    ctx.commit("root.txt", "initial content", "root commit");

    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("f1.txt", "f1", "feature commit");

    // Modify root.txt in feature to be different from base-branch
    ctx.commit("root.txt", "feature content", "feature modify root");

    let (success, _, _) =
        ctx.run_staircase(&["adopt", "my-staircase", "feature", "--onto", "base-branch"]);
    assert!(success);

    // Start on base-branch
    ctx.run_git(&["checkout", "base-branch"]);

    // ACT: Run verify. It will checkout the 'feature' cut.
    // Build command modifies root.txt to something different from BOTH branches.
    let (success, stdout, stderr) = ctx.run_staircase(&[
        "verify",
        "my-staircase",
        "--build-command",
        "echo dirty > root.txt",
    ]);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    assert!(success);

    let current_branch = ctx.run_git(&["rev-parse", "--abbrev-ref", "HEAD"]);
    println!("Current branch: {}", current_branch);

    // Check if we are back on 'base-branch'
    if current_branch == "HEAD" || current_branch != "base-branch" {
        panic!(
            "BUG REPRODUCED: Failed to return to 'base-branch' branch! Current branch: {}",
            current_branch
        );
    }

    panic!("Bug not reproduced. Current branch is {}", current_branch);
}

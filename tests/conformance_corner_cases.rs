mod common;
use common::TestContext;

#[test]
fn main_ahead_of_anchor_is_valid_work() {
    let ctx = TestContext::new();

    // ARRANGE: Create a repository where 'main' has 2 commits ahead of the anchor.
    // Initial commit was already created by TestContext::new() on branch 'main'.
    let anchor_oid = ctx.run_git(&["rev-parse", "main"]);

    // Create origin/main branch to serve as integration anchor
    ctx.run_git(&["update-ref", "refs/remotes/origin/main", &anchor_oid]);

    // Add 2 commits to main
    ctx.commit("file1.txt", "content1", "feature 1");
    ctx.commit("file2.txt", "content2", "feature 2");

    // ACT: Run 'git staircase list' with the anchor explicitly specified
    // to verify that discovery logic considers 'main' as a candidate.
    let (ok, stdout, stderr) = ctx.run_staircase(&["list", "--onto", "origin/main"]);

    // ASSERT: Verify that 'main' appears in the list as a discoverable implicit staircase.
    assert!(ok, "list failed: {}", stderr);
    assert!(
        stdout.contains("main") && stdout.contains("(implicit)"),
        "main should be discovered as an implicit staircase when ahead of anchor.\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn main_ahead_of_non_origin_anchor_is_valid_work() {
    let ctx = TestContext::new();

    // ARRANGE: Create a repository where 'main' has 2 commits ahead of 'upstream/main'.
    let anchor_oid = ctx.run_git(&["rev-parse", "main"]);

    // Create upstream/main branch (local branch for simplicity in test)
    ctx.run_git(&["branch", "upstream/main", &anchor_oid]);

    // Add 2 commits to main
    ctx.commit("file1.txt", "content1", "feature 1");
    ctx.commit("file2.txt", "content2", "feature 2");

    // Set upstream for main to upstream/main
    ctx.run_git(&["branch", "--set-upstream-to=upstream/main", "main"]);

    // switch to a different branch so step 2 of infer_onto is skipped
    ctx.run_git(&["checkout", "-b", "other"]);

    // ACT: Run 'git staircase list'
    let (ok, stdout, stderr) = ctx.run_staircase(&["list"]);

    // ASSERT: Verify that 'main' appears in the list.
    assert!(ok, "list failed: {}", stderr);
    assert!(
        stdout.contains("main"),
        "main should be discovered when ahead of upstream/main even if not on main branch.\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );
}

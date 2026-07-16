mod common;
use common::TestContext;
use std::fs;

#[test]
fn test_repro_archive_rollback_failure() {
    let ctx = TestContext::new();
    let path = ctx.path();

    // 1. Create a staircase
    ctx.commit("file1.txt", "content1", "commit 1");
    ctx.run_git(&["branch", "b1"]);
    ctx.run_staircase(&["adopt", "--onto", "HEAD^", "my-sc", "b1"]);

    // 2. Create another worktree and check out b1
    let wt_path = path.join("wt1");
    ctx.run_git(&["worktree", "add", wt_path.to_str().unwrap(), "b1"]);

    // 3. Make the archive process fail by creating a ref collision
    // We'll create a ref that archive_staircase expects to not exist or be different.
    // Wait, the easiest way to make it fail is to create a 'record-publication' plan failure.
    // MutationPlan::publish fails if a ref collision occurs.

    // Archive wants to update refs/staircases/my-sc (public ref).
    // Let's change it to something else after archive_staircase reads it but before it publishes.
    // This is hard to time in a single process.

    // Alternatively, look at archive.rs:172. It fails if a ref is missing.
    // But it detaches worktrees at line 161.
    // So if we make it fail at line 172, worktrees at 161 are already detached.

    // To fail at line 172, we need owned_branches to contain a ref that doesn't exist.
    // But owned_branches is built from meta.steps.

    // How about if we delete a branch after it's been identified as owned but before line 172?
    // Still timing dependent.

    // Let's use a simpler way: worktree_repo.run at 161 might fail if the worktree is dirty
    // and we don't pass --detach-dirty-worktrees.
    // But archive_staircase checks dirty state at 135 and returns early.

    // Wait! Line 132: 'let worktree_repo = GitRepo::new(worktree.path.clone());'
    // Line 161: 'worktree_repo.run(&["checkout", "--detach", &step_cut])?;'

    // If we have MULTIPLE worktrees. The first one is detached, the second one fails.

    let wt2_path = path.join("wt2");
    ctx.run_git(&["branch", "b2"]);
    ctx.run_git(&["worktree", "add", wt2_path.to_str().unwrap(), "b2"]);
    // Redefine staircase to include both
    ctx.run_staircase(&["adopt", "--onto", "HEAD^", "my-sc", "b1", "b2"]);

    // Make wt2 dirty and DON'T pass --detach-dirty-worktrees.
    fs::write(wt2_path.join("dirty.txt"), "dirty").unwrap();

    // Now call archive. It should fail because wt2 is dirty.
    let (success, stdout, stderr) = ctx.run_staircase(&["archive", "my-sc"]);
    assert!(
        !success,
        "Archive should have failed due to dirty wt2: {} {}",
        stdout, stderr
    );

    // 4. Verify WT1 is detached even though archiving failed
    let wt1_list = ctx.run_git(&[
        "-C",
        wt_path.to_str().unwrap(),
        "rev-parse",
        "--abbrev-ref",
        "HEAD",
    ]);
    println!("WT1 HEAD: {}", wt1_list);

    // If WT1 was detached, rev-parse --abbrev-ref HEAD returns "HEAD"
    if wt1_list == "HEAD" {
        panic!("REPRODUCED: WT1 was detached even though archive failed!");
    }
}

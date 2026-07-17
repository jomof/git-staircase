mod common;
use common::*;
use std::path::PathBuf;

#[test]
fn test_worktrees_correctly_parses_multiple_worktrees() {
    // ARRANGE
    let ctx = TestContext::new();
    let root_path = ctx.path().to_path_buf();

    // Create a second worktree
    let wt_path = ctx.tmp.path().join("wt2");
    ctx.run_git(&["branch", "wt-branch"]);
    ctx.run_git(&["worktree", "add", wt_path.to_str().unwrap(), "wt-branch"]);

    // ACT
    let worktrees = ctx.repo.worktrees().expect("Failed to list worktrees");

    // ASSERT
    // We expect 2 worktrees: the main one and the new one.
    assert_eq!(
        worktrees.len(),
        2,
        "Should have found 2 worktrees, found: {:?}",
        worktrees
    );

    // Check if both paths are present
    let paths: Vec<PathBuf> = worktrees
        .iter()
        .map(|w| w.path.canonicalize().unwrap())
        .collect();
    let expected_root = root_path.canonicalize().unwrap();
    let expected_wt = wt_path.canonicalize().unwrap();

    assert!(
        paths.contains(&expected_root),
        "Root path {:?} not in {:?}",
        expected_root,
        paths
    );
    assert!(
        paths.contains(&expected_wt),
        "Worktree path {:?} not in {:?}",
        expected_wt,
        paths
    );
}

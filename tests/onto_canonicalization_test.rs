mod common;
use common::*;

#[test]
fn test_adopt_canonicalizes_onto() {
    // ARRANGE
    let ctx = TestContext::new();

    // Create a branch to adopt
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("file.txt", "content", "feature commit");

    // Ensure 'main' exists (setup_repo creates it)
    ctx.run_git(&["rev-parse", "--verify", "refs/heads/main"]);

    // ACT: Adopt with --onto main (abbreviated name)
    let (success, _stdout, stderr) =
        ctx.run_staircase(&["adopt", "my-staircase", "--onto", "main", "feature"]);
    assert!(success, "Adopt failed: {}", stderr);

    // ASSERT: Check generation-1 structure for the full symbolic target.
    let content = ctx.run_git(&["cat-file", "-p", "refs/staircases/my-staircase:structure"]);
    let structure: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        structure["integration_context"]["symbolic_targets"][0],
        "refs/heads/main"
    );
}

#[test]
fn test_infer_onto_canonicalizes() {
    // ARRANGE
    let ctx = TestContext::new();

    // Create a branch and set its upstream to main
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.run_git(&["branch", "--set-upstream-to=main"]);
    ctx.commit("file.txt", "content", "feature commit");

    // ACT: Adopt without --onto, it should infer from upstream
    let (success, _stdout, stderr) = ctx.run_staircase(&["adopt", "my-staircase", "feature"]);
    assert!(success, "Adopt failed: {}", stderr);

    // ASSERT: Check generation-1 structure for the inferred full refname.
    let content = ctx.run_git(&["cat-file", "-p", "refs/staircases/my-staircase:structure"]);
    let structure: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        structure["integration_context"]["symbolic_targets"][0],
        "refs/heads/main"
    );
}

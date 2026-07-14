mod common;
use common::TestContext;

#[test]
fn test_symbolic_target_is_lost_after_adopt() {
    let ctx = TestContext::new();

    // ARRANGE: Create a repo with a target branch
    ctx.run_git(&["checkout", "-b", "target-branch"]);
    ctx.commit("root.txt", "root", "root commit");

    // Create a feature branch
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("f1.txt", "f1", "feature commit");

    // ACT: Adopt the feature branch using 'target-branch' as target
    let (success, _, _) = ctx.run_staircase(&[
        "adopt",
        "my-staircase",
        "feature",
        "--onto",
        "target-branch",
    ]);
    assert!(success);

    // Now check the metadata
    let (success, stdout, _) = ctx.run_staircase(&["show", "my-staircase", "--json"]);
    assert!(success);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let target = json.get("target").and_then(|v| v.as_str()).unwrap();

    // ASSERT: Target should be 'target-branch', not an OID
    if target.len() == 40 || target.len() == 64 {
        panic!(
            "BUG REPRODUCED: Target was pinned to OID '{}' instead of symbolic name 'target-branch'!",
            target
        );
    }
    assert_eq!(target, "target-branch");
}

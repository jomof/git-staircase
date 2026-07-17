
use crate::common::*;

#[test]
fn test_resolve_commit_non_existent_oid() {
    // ARRANGE
    let ctx = TestContext::new();
    let fake_oid = "a".repeat(40);

    // ACT
    let res = ctx.repo.resolve_commit(&fake_oid);

    // ASSERT
    assert!(
        res.is_err(),
        "Should have failed for non-existent OID but got Ok"
    );
}

#[test]
fn test_resolve_commit_branch_named_like_oid() {
    // ARRANGE
    let ctx = TestContext::new();
    let branch_name = "b".repeat(40);
    ctx.run_git(&["checkout", "-b", &branch_name]);
    let head_oid = ctx.run_git(&["rev-parse", "HEAD"]);

    // ACT
    let res = ctx.repo.resolve_commit(&branch_name).unwrap();

    // ASSERT
    assert_eq!(res, head_oid, "Should have resolved to head OID of branch");
}

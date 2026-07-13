mod common;
use common::*;

#[test]
fn test_is_ancestor_stale_cache() {
    // ARRANGE
    let ctx = TestContext::new();
    let oid1 = ctx.commit("file1.txt", "content1", "commit 1");
    ctx.run_git(&["branch", "feature", &oid1]);

    // Initial check: main (oid1) is ancestor of feature (oid1) -> true
    assert!(ctx.repo.is_ancestor("main", "feature").unwrap());

    // Now move main forward, so it's no longer an ancestor of feature
    let _oid2 = ctx.commit("file2.txt", "content2", "commit 2");

    // ACT
    let res = ctx.repo.is_ancestor("main", "feature").unwrap();

    // ASSERT
    assert!(
        !res,
        "Should have returned false because main moved forward, but got true (stale cache)"
    );
}

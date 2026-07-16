mod common;
use common::TestContext;

#[test]
#[ignore]
fn test_repro_cache_bottleneck() {
    let ctx = TestContext::new();

    // 1. Memoize something immutable
    let head = ctx.run_git(&["rev-parse", "HEAD"]);
    let _tree_id = ctx.repo.get_tree_id(&head).unwrap();

    // Verify it's memoized
    assert!(ctx.repo.memoizer.TreeId(head.clone()).get().is_some());

    // 2. Update an unrelated branch
    ctx.run_git(&["branch", "other"]);
    ctx.repo.update_branch("other", &head).unwrap();

    // 3. Verify that the unrelated TreeId is GONE
    let tree_id_after = ctx.repo.memoizer.TreeId(head.clone()).get();

    if tree_id_after.is_none() {
        panic!("REPRODUCED: Bottleneck: update_branch cleared unrelated immutable cache entry!");
    }
}

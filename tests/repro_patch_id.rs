mod common;
use common::*;

#[test]
fn test_patch_id_stale_cache() {
    let ctx = TestContext::new();
    let oid1 = ctx.commit("file1.txt", "content1", "commit 1");
    let _oid2 = ctx.commit("file1.txt", "content2", "commit 2");

    // Initial patch-id between oid1 and oid2 (on main)
    let pid1 = ctx.repo.get_patch_id(&oid1, "main").unwrap();

    // Move main forward
    let _oid3 = ctx.commit("file1.txt", "content3", "commit 3");

    // Now patch-id between oid1 and main (oid3) should be different
    let pid2 = ctx.repo.get_patch_id(&oid1, "main").unwrap();

    assert_ne!(pid1, pid2, "Patch-id should have changed when main moved");
}

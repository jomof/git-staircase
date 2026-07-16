mod common;
use common::*;

#[test]
fn test_stale_resolve_commit() {
    // ARRANGE
    let (tmp, repo) = setup_repo();
    let dir = tmp.path();

    // Initial commit (oid1) is already created by setup_repo on 'main'
    let oid1 = get_head_oid(dir);

    // Resolve 'main' - this will cache it
    let resolved1 = repo.resolve_commit("main").unwrap();
    assert_eq!(resolved1, oid1);

    // Create a second commit (oid2)
    // We use raw git command to avoid clearing the repo's memoizer
    run_git(dir, &["commit", "--allow-empty", "-m", "second commit"]);
    let oid2 = get_head_oid(dir);
    assert_ne!(oid1, oid2);

    // ACT
    // Resolve 'main' again. It SHOULD be oid2 now.
    let resolved2 = repo.resolve_commit("main").unwrap();

    // ASSERT
    assert_eq!(
        resolved2, oid2,
        "Resolve 'main' returned stale OID {} instead of new OID {}",
        resolved2, oid2
    );
}

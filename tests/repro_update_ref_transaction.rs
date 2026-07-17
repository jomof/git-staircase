mod common;
use common::*;

#[test]
fn test_update_refs_transaction_fails_on_conflict() {
    // ARRANGE
    let ctx = TestContext::new();
    let oid1 = ctx.commit("file1.txt", "content1", "commit 1");
    let oid2 = ctx.commit("file2.txt", "content2", "commit 2");

    // Create two conflicting update-ref commands in a transaction.
    // update-ref --stdin format for commands is:
    // update <ref> <newvalue> [<oldvalue>]
    let commands = vec![
        format!("update refs/heads/branch-a {}", oid1),
        format!("update refs/heads/branch-a {}", oid2),
    ];

    // ACT
    let result = ctx.repo.update_refs_transaction(&commands);

    // ASSERT
    // This should fail because we are updating the same ref twice in one transaction.
    assert!(
        result.is_err(),
        "Transaction should have failed due to conflicting updates to the same ref"
    );
}

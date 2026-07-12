mod common;
use common::*;
use git_staircase::core::resolve_staircase;

#[test]
#[ignore]
fn test_id_consistency_when_resolving_prefix() {
    // ARRANGE
    let ctx = TestContext::new();

    // Create two branches: A and B, where B is on top of A
    ctx.run_git(&["checkout", "-b", "branchA"]);
    ctx.commit("a.txt", "a", "A");

    ctx.run_git(&["checkout", "-b", "branchB"]);
    ctx.commit("b.txt", "b", "B");

    // ACT: Resolve by name "branchA". This should give a staircase with only branchA.
    let resolved = resolve_staircase(&ctx.repo, "branchA", Some("main")).unwrap().unwrap();
    let meta = resolved.metadata();
    
    // ASSERT
    let object_format = ctx.repo.get_object_format().unwrap();
    let onto_oid = ctx.repo.resolve_commit("main").unwrap();
    let expected_id = git_staircase::core::discovery::compute_implicit_id(&object_format, &onto_oid, &meta.steps);
    
    assert_eq!(meta.id, expected_id, "In-memory metadata ID should match the structural ID");
}

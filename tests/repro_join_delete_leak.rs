mod common;
use common::*;
use git_staircase::core;
use git_staircase::core::manipulation::{JoinOptions, JoinRefAction};

#[test]
fn test_join_delete_leak() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "step1"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "step2"]);
    let c2 = ctx.commit("file2.txt", "2", "commit 2");

    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let managed = core::adopt(&ctx.repo, &s).unwrap();

    let rs = git_staircase::ResolvedSelector {
        staircase: core::resolve_by_id(&ctx.repo, &managed.id).unwrap(),
        step_index: None,
    };

    // ACT
    // Join step 1 and step 2, and request DELETE for the retired boundary ref (step1)
    core::join(
        &ctx.repo,
        &rs,
        0,
        1,
        JoinOptions {
            ref_action: JoinRefAction::Delete,
        },
    )
    .unwrap();

    // ASSERT
    let read = core::persistence::read_metadata(&ctx.repo, &managed.id).unwrap();
    assert_eq!(read.steps.len(), 1, "Should have only 1 step after join");
    assert_eq!(read.steps[0].cut, c2);

    // Check if the retired branch 'step1' is still there
    let step1_ref = ctx.repo.resolve_ref_opt("refs/heads/step1").unwrap();
    assert!(
        step1_ref.is_none(),
        "Branch 'step1' should have been deleted, but it is still there at {:?}",
        step1_ref
    );
}

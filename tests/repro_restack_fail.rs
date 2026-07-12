mod common;
use common::*;
use git_staircase::*;

#[test]
fn test_restack_conflict_handling() {
    // ARRANGE
    let ctx = TestContext::new();

    // 1. Create a chain: s1
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("conflict.txt", "s1", "s1 commit");

    // 2. Adopt as a staircase
    let sc = StaircaseMetadata {
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
        ],
        verification_policy: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();

    // 3. Modify main to conflict with s1
    ctx.run_git(&["checkout", "main"]);
    ctx.commit("conflict.txt", "main", "main update");

    // 4. Resolve and try to restack
    let rs = git_staircase::core::resolve_staircase(&ctx.repo, "test", None).unwrap().unwrap();
    
    // ACT
    let result = git_staircase::core::restack(&ctx.repo, &rs);

    // ASSERT
    assert!(result.is_err(), "Restack should fail when there is a conflict");
}

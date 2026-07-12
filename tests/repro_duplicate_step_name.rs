mod common;
use common::*;
use git_staircase::ResolvedStaircase;
use git_staircase::model::{StaircaseMetadata, Step};

#[test]
#[ignore]
fn test_split_duplicate_name() {
    let ctx = TestContext::new();

    // 1. Create a chain: main -> c1 -> c2
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("1.txt", "1", "c1");
    let c2 = ctx.commit("2.txt", "2", "c2");

    // 2. Adopt as managed staircase with step s1 at c2
    let sc = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: "s1-id".to_string(),
            name: "s1".to_string(),
            cut: c2.clone(),
            branch: Some("s1".to_string()),
        }],
        verification_policy: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();
    let rs = ResolvedStaircase::Managed(sc);

    // 3. Split at c1 with name "s1" (which already exists)
    let result = git_staircase::core::manipulation::split(
        &ctx.repo,
        &rs,
        0,
        &c1,
        Some("s1"),
        git_staircase::core::SplitOptions { no_ref: false },
    );

    // FAILURE: Result is Ok(()), meaning a duplicate name was allowed
    assert!(
        result.is_err(),
        "Splitting with an existing step name should fail. Result: {:?}",
        result
    );
}

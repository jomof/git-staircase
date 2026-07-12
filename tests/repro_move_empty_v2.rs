mod common;
use common::*;
use git_staircase::core::persistence;
use git_staircase::core::{ResolvedStaircase, adopt, manipulation};
use git_staircase::model::{StaircaseMetadata, Step};

#[test]
fn test_move_commit_creates_empty_step_violating_invariant() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let c1 = ctx.commit("f1.txt", "1", "c1");
    let c2 = ctx.commit("f2.txt", "2", "c2");

    let meta = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: target,
        steps: vec![
            Step {
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: None,
    };
    let rs = ResolvedStaircase::Managed(meta.clone());
    persistence::write_metadata(&ctx.repo, &meta).unwrap();

    // ACT: Move the only commit of s2 into s1
    manipulation::move_commits(&ctx.repo, &rs, 1, 0, &[c2.clone()]).unwrap();

    // ASSERT: The resulting staircase is now considered invalid by the core logic
    let updated_meta = persistence::read_metadata(&ctx.repo, "test-id").unwrap();
    let result = adopt(&ctx.repo, &updated_meta);
    assert!(
        result.is_err(),
        "Staircase with empty step should be rejected by validation logic"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("every step must be non-empty")
    );
}

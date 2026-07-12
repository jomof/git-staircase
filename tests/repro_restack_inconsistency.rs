mod common;
use common::*;

use git_staircase::core::persistence;
use git_staircase::model::{StaircaseMetadata, Step};

#[test]
#[ignore]
fn test_restack_inconsistency_on_failure() {
    let ctx = TestContext::new();
    
    // 1. Create a chain: main -> s1 -> s2
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("s1.txt", "s1", "s1 commit");
    ctx.run_git(&["checkout", "-b", "s2"]);
    let c2 = ctx.commit("s2.txt", "s2", "s2 commit");
    
    // 2. Adopt as a managed staircase
    let sc = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: "s1-id".to_string(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                id: "s2-id".to_string(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();
    
    // 3. Update main with a commit that conflicts with s2 but NOT s1
    ctx.run_git(&["checkout", "main"]);
    ctx.commit("main-unrelated.txt", "main", "unrelated update");
    ctx.commit("s2.txt", "conflict", "conflict with s2");
    
    let rs = git_staircase::core::resolve_staircase(&ctx.repo, "test", None).unwrap().unwrap();
    
    // 4. Run restack. It should succeed on s1 and fail on s2.
    let result = git_staircase::core::manipulation::restack(&ctx.repo, &rs);
    assert!(result.is_err(), "Restack should fail on s2 conflict");
    
    // 5. Check consistency
    let s1_branch_oid = ctx.repo.resolve_commit("s1").unwrap();
    let meta = persistence::read_metadata(&ctx.repo, "test-id").unwrap();
    let s1_meta_oid = meta.steps[0].cut.clone();
    
    // FAILURE: s1_branch_oid is c1 (rolled back), but s1_meta_oid is the new rebased OID
    assert_eq!(s1_branch_oid, s1_meta_oid, "Branch and metadata should be consistent after failed restack");
}

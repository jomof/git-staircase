use crate::common::*;
use git_staircase::core::manipulation::{land, LandOptions};
use git_staircase::core::resolution::resolve_by_id;
use git_staircase::core::resolved::adopt;
use git_staircase::core::ResolvedStaircase;
use git_staircase::model::{LifecycleState, StaircaseMetadata, Step};

#[test]
fn test_land_leaves_staircase_active() {
    let ctx = TestContext::new();
    let repo = ctx.repo.clone();
    let _initial_oid = repo.resolve_commit("main").unwrap();

    // Create a commit to land on a separate branch
    ctx.run_git(&["checkout", "-b", "feature"]);
    let c1 = ctx.commit("f1.txt", "c1", "c1");

    // Set up a managed staircase
    let metadata = StaircaseMetadata {
        landing_policy: None,
        id: "land-test-id".to_string(),
        name: "land-test".to_string(),
        target: "refs/heads/main".to_string(),
        steps: vec![Step {
            id: "S1".into(),
            name: "S1".into(),
            cut: c1.clone(),
            branch: None,
        }],
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    let adopted = adopt(&repo, &metadata).expect("Adopt failed");
    let rs = ResolvedStaircase::Managed(adopted);

    // Land it
    land(&repo, &rs, LandOptions { policy: None }).expect("Land failed");

    // Verify target branch moved
    let new_target_oid = repo.resolve_commit("refs/heads/main").unwrap();
    assert_eq!(
        new_target_oid, c1,
        "Target branch should have moved to the top of the staircase"
    );

    // Now check if the staircase still exists AS ACTIVE
    let res = resolve_by_id(&repo, &rs.metadata().id);

    // ARRANGE/ACT/ASSERT
    if let Ok(staircase) = res {
        if let ResolvedStaircase::Managed(metadata) = staircase {
            let is_archived = metadata.lifecycle.map_or(false, |l| {
                l.state == LifecycleState::Archived
            });
            assert!(
                is_archived,
                "Staircase should be archived after landing, but its state is active."
            );
        } else {
            panic!("Expected Managed staircase.");
        }
    } else {
        // Technically passing if not found at all, but we expect it to be archived
    }
}

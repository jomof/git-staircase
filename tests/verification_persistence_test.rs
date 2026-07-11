use git_staircase::core::persistence;
mod common;
use common::*;
use git_staircase::model::{StaircaseMetadata, Step, VerificationPolicy};

#[test]
fn test_verification_policy_persistence() {
    // ARRANGE: Create a StaircaseMetadata with a VerificationPolicy
    let (_tmp, repo) = setup_repo();

    let policy = VerificationPolicy {
        build_command: Some("make build".to_string()),
        test_command: Some("make test".to_string()),
        verify_each_prefix: true,
    };

    let metadata = StaircaseMetadata {
        id: "test-id-123".to_string(),
        name: "test-staircase".to_string(),
        target: "refs/heads/main".to_string(),
        steps: vec![Step {
            name: "step1".to_string(),
            cut: repo.resolve_ref("HEAD").unwrap(),
            branch: None,
        }],
        verification_policy: Some(policy.clone()),
    };

    // ACT: Write and then read the metadata
    persistence::write_metadata(&repo, &metadata).expect("Failed to write metadata");
    let read_meta =
        persistence::read_metadata(&repo, "test-staircase").expect("Failed to read metadata");

    // ASSERT: Verify the policy is preserved
    assert_eq!(
        read_meta.verification_policy,
        Some(policy),
        "Verification policy should be preserved"
    );
}

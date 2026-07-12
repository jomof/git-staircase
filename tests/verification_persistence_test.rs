use git_staircase::core::persistence;
mod common;
use common::*;
use git_staircase::model::{
    IdentityKind, StaircaseMetadata, Step, VerificationPolicy, VerificationResult,
};

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
        landing_policy: None,
        id: "test-id-123".to_string(),
        name: "test-staircase".to_string(),
        target: "refs/heads/main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "step1".to_string(),
            cut: repo.resolve_ref("HEAD").unwrap(),
            branch: None,
        }],
        verification_policy: Some(policy.clone()),

        primary_branch_layout: None,
        branch_layout_base: None,
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

#[test]
fn test_revision_verification_results_persistence() {
    let (_tmp, repo) = setup_repo();
    let cut = repo.resolve_ref("HEAD").unwrap();

    let results = vec![
        VerificationResult {
            step_name: "step1".to_string(),
            cut: cut.clone(),
            success: true,
            stdout: "Build success".to_string(),
            stderr: "".to_string(),
        },
        VerificationResult {
            step_name: "step2".to_string(),
            cut: cut.clone(),
            success: false,
            stdout: "Test failure".to_string(),
            stderr: "Error: assertion failed".to_string(),
        },
    ];

    let revision_key = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    // ACT: Record
    persistence::record_verification(&repo, revision_key, IdentityKind::Revision, &results)
        .expect("Failed to record verification");

    // ACT: Read
    let read_results = persistence::read_verification(&repo, revision_key, IdentityKind::Revision)
        .expect("Failed to read verification")
        .expect("Verification results should exist");

    // ASSERT
    assert_eq!(read_results, results);

    // Also verify the ref exists in the expected format (with slash instead of colon)
    let expected_ref = format!(
        "refs/staircases/by-revision/{}/verification",
        revision_key.replace(":", "/")
    );
    assert!(
        repo.resolve_ref(&expected_ref).is_ok(),
        "Ref {} should exist",
        expected_ref
    );
}

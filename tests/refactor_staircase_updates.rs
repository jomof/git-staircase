mod common;
use common::*;
use git_staircase::core;
use git_staircase::core::ResolvedStaircase;
use git_staircase::model::{StaircaseMetadata, Step};

fn setup_complex_repo() -> TestContext {
    let ctx = TestContext::new();

    // Step A
    ctx.run_git(&["checkout", "-b", "branch-a"]);
    ctx.commit("f", "A1\n", "commit A1");
    ctx.commit("f", "A2\n", "commit A2");

    // Step B
    ctx.run_git(&["checkout", "-b", "branch-b"]);
    ctx.commit("f", "B\n", "commit B");

    ctx
}

#[test]
fn test_managed_staircase_updates() {
    // ARRANGE
    let ctx = setup_complex_repo();
    let a2_oid = ctx.repo.resolve_ref("branch-a").unwrap();
    let a1_oid = ctx.repo.resolve_ref("branch-a~1").unwrap();
    let b_oid = ctx.repo.resolve_ref("branch-b").unwrap();

    let metadata = StaircaseMetadata {
        id: "test-managed".to_string(),
        name: "Managed".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "A".to_string(),
                cut: a2_oid.clone(),
                branch: Some("branch-a".to_string()),
            },
            Step {
                name: "B".to_string(),
                cut: b_oid.clone(),
                branch: Some("branch-b".to_string()),
            },
        ],
        verification_policy: None,
    };
    core::adopt(&ctx.repo, &metadata).unwrap();

    let rs = ResolvedStaircase::Managed(metadata);

    // ACT (Split Step A)
    core::split(&ctx.repo, &rs, 0, &a1_oid, Some("A-part1")).unwrap();

    // ASSERT (Split)
    let rs = core::resolve_staircase(&ctx.repo, "test-managed", None)
        .unwrap()
        .unwrap();
    assert_eq!(rs.metadata().steps.len(), 3);
    assert_eq!(rs.metadata().steps[0].name, "A-part1");
    assert!(
        ctx.repo.resolve_ref(&format!(
            "refs/staircase-state/{}/steps/A-part1",
            rs.metadata().id
        ))
        .is_ok()
    );

    // ACT (Join Step A-part1 and A)
    core::join(&ctx.repo, &rs, 0, 1).unwrap();

    // ASSERT (Join)
    let rs = core::resolve_staircase(&ctx.repo, "test-managed", None)
        .unwrap()
        .unwrap();
    assert_eq!(rs.metadata().steps.len(), 2);
    assert!(
        ctx.repo.resolve_ref(&format!(
            "refs/staircase-state/{}/steps/A-part1",
            rs.metadata().id
        ))
        .is_err()
    );

    // ACT (Restack)
    core::restack(&ctx.repo, &rs).unwrap();
}

#[test]
fn test_implicit_staircase_updates() {
    // ARRANGE
    let ctx = setup_complex_repo();
    let a2_oid = ctx.repo.resolve_ref("branch-a").unwrap();
    let a1_oid = ctx.repo.resolve_ref("branch-a~1").unwrap();
    let b_oid = ctx.repo.resolve_ref("branch-b").unwrap();

    let metadata = StaircaseMetadata {
        id: "test-implicit".to_string(),
        name: "Implicit".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "A".to_string(),
                cut: a2_oid.clone(),
                branch: Some("branch-a".to_string()),
            },
            Step {
                name: "B".to_string(),
                cut: b_oid.clone(),
                branch: Some("branch-b".to_string()),
            },
        ],
        verification_policy: None,
    };

    let rs = ResolvedStaircase::Implicit(metadata);

    // ACT
    core::split(&ctx.repo, &rs, 0, &a1_oid, Some("branch-a-part1")).unwrap();

    // ASSERT
    assert!(ctx.repo.resolve_ref("branch-a-part1").is_ok());
}

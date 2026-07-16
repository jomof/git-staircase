mod common;
use anyhow::Result;
use common::*;
use git_staircase::core;

#[test]
fn test_adopt_by_implicit_name() -> Result<()> {
    // ARRANGE: Create a 3-step implicit staircase in a repo.
    let ctx = TestContext::new();

    // Create base commit (setup_repo already does this)

    // Create 3 branches forming a stack
    ctx.run_git(&["checkout", "-b", "feature-1"]);
    let oid1 = ctx.commit("f1", "1", "feature 1");

    ctx.run_git(&["checkout", "-b", "feature-2"]);
    let oid2 = ctx.commit("f2", "2", "feature 2");

    ctx.run_git(&["checkout", "-b", "feature-3"]);
    let oid3 = ctx.commit("f3", "3", "feature 3");

    // ACT: Run `git staircase adopt feature-3`.
    // We simulate the CLI call here.
    let adopt_cmd = git_staircase::cli::adopt::Adopt {
        name: "feature-3".to_string(),
        onto: None,
        branches: vec![], // Empty branches means use selector
        build_command: None,
        test_command: None,
        landing_policy: None,
        verify_each_prefix: false,
    };

    use git_staircase::cli::Command;
    let result = adopt_cmd.run(&ctx.repo);

    // ASSERT: Verify it succeeded (currently it should fail because branches is empty)
    assert!(
        result.is_ok(),
        "Adopt by selector should succeed, but failed: {:?}",
        result.err()
    );

    // ASSERT: Verify the staircase is now managed and preserves the same structure
    let managed = core::persistence::list_all_staircases(&ctx.repo)?;
    assert_eq!(managed.len(), 1);
    let s = &managed[0];
    assert_eq!(s.name, "feature-3");
    assert_eq!(s.steps.len(), 3);
    assert_eq!(s.steps[0].cut, oid1);
    assert_eq!(s.steps[1].cut, oid2);
    assert_eq!(s.steps[2].cut, oid3);

    Ok(())
}

#[test]
fn test_adopt_non_existent_selector_fails() -> Result<()> {
    let ctx = TestContext::new();

    let adopt_cmd = git_staircase::cli::adopt::Adopt {
        name: "non-existent".to_string(),
        onto: None,
        branches: vec![],
        build_command: None,
        test_command: None,
        landing_policy: None,
        verify_each_prefix: false,
    };

    use git_staircase::cli::Command;
    let result = adopt_cmd.run(&ctx.repo);

    // ASSERT: Verify that trying to adopt a non-existent selector fails with a diagnostic.
    assert!(result.is_err());
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(
        err_msg.contains("not found") || err_msg.contains("non-existent"),
        "Error message should be diagnostic, got: {}",
        err_msg
    );

    Ok(())
}

#[test]
fn test_adopt_structural_binding_fails_if_branch_moved() -> Result<()> {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature-1"]);
    ctx.commit("f1", "1", "feature 1");

    // Resolve it first to get the metadata with the OID
    let repo = &ctx.repo;
    let resolved = core::resolve_staircase(repo, "feature-1", None)?.unwrap();
    let metadata = resolved.staircase.metadata().clone();

    // Now move the branch
    ctx.commit("f1", "2", "feature 1 moved");

    // Try to adopt the OLD metadata
    let result = core::adopt(repo, &metadata);

    // ASSERT: It should fail because the branch moved
    assert!(result.is_err());
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(
        err_msg.contains("moved") && err_msg.contains("structural binding failed"),
        "Error message should mention structural binding failure, got: {}",
        err_msg
    );

    Ok(())
}

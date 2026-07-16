mod common;
use common::*;
use git_staircase::core;
use git_staircase::{Discovery, ResolvedStaircase, StaircaseMetadata, Step, VerificationPolicy};
use std::fs;

#[test]
fn test_discover_linear() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "feature/auth-ui"]);
    let c2 = ctx.commit("file2.txt", "2", "commit 2");

    ctx.run_git(&["checkout", "-b", "feature/auth-tests"]);
    let c3 = ctx.commit("file3.txt", "3", "commit 3");

    // ACT
    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();

    // ASSERT
    assert_eq!(discovered.len(), 1);
    let Discovery::Linear(ref s) = discovered[0] else {
        panic!("Expected linear discovery");
    };
    assert_eq!(s.name, "feature/auth");
    assert_eq!(s.target, "refs/heads/main");
    assert_eq!(s.steps.len(), 3);

    assert_eq!(s.steps[0].name, "feature/auth-core");
    assert_eq!(s.steps[0].cut, c1);

    assert_eq!(s.steps[1].name, "feature/auth-ui");
    assert_eq!(s.steps[1].cut, c2);

    assert_eq!(s.steps[2].name, "feature/auth-tests");
    assert_eq!(s.steps[2].cut, c3);
}

#[test]
fn test_adopt_and_status() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "feature/auth-ui"]);
    let _c2 = ctx.commit("file2.txt", "2", "commit 2");

    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();

    // ACT
    let s = core::adopt(&ctx.repo, &s).unwrap();

    // ASSERT
    let read = core::persistence::read_metadata(&ctx.repo, &s.id).unwrap();
    assert_eq!(read.name, "auth");
    assert_eq!(read.steps.len(), 2);

    let status = core::get_status(&ctx.repo, &s.id).unwrap();
    assert!(status.is_clean);

    // ACT (Modify)
    ctx.run_git(&["checkout", "feature/auth-ui"]);
    let c2_new = ctx.commit("file2_mod.txt", "2 mod", "commit 2 mod");

    // ASSERT (Stale)
    let status = core::get_status(&ctx.repo, &s.id).unwrap();
    assert!(!status.is_clean);
    assert_eq!(status.steps[1].actual_oid, Some(c2_new.clone()));

    let found = core::resolve_by_id(&ctx.repo, &s.id).unwrap();
    assert_eq!(found.metadata().name, "auth");
}

#[test]
fn test_status_stale_and_restack() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "feature/auth-ui"]);
    let c2 = ctx.commit("file2.txt", "2", "commit 2");

    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let s = core::adopt(&ctx.repo, &s).unwrap();

    ctx.run_git(&["checkout", "feature/auth-core"]);
    fs::write(ctx.path().join("file1.txt"), "1 amended").unwrap();
    ctx.run_git(&["add", "."]);
    ctx.run_git(&["commit", "--amend", "-m", "commit 1 amended"]);
    let c1_amended = ctx.repo.resolve_ref("HEAD").unwrap();

    // ACT
    let status = core::get_status(&ctx.repo, &s.id).unwrap();
    assert!(!status.is_clean);

    let rs = git_staircase::ResolvedSelector {
        staircase: core::resolve_by_id(&ctx.repo, &s.id).unwrap(),
        step_index: None,
    };
    core::restack(
        &ctx.repo,
        &rs,
        core::RebaseOptions {
            leave_upper_steps_stale: false,
        },
    )
    .unwrap();

    // ASSERT
    let status = core::get_status(&ctx.repo, &s.id).unwrap();
    assert!(status.is_clean);
    assert_eq!(status.steps[0].actual_oid, Some(c1_amended));
    assert_ne!(status.steps[1].actual_oid, Some(c2));
}

#[test]
fn test_split_and_join() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");
    let c1_2 = ctx.commit("file1_2.txt", "1.2", "commit 1.2");
    let c1_3 = ctx.commit("file1_3.txt", "1.3", "commit 1.3");

    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let s = core::adopt(&ctx.repo, &s).unwrap();

    let rs = git_staircase::ResolvedSelector {
        staircase: core::resolve_by_id(&ctx.repo, &s.id).unwrap(),
        step_index: None,
    };

    // ACT (Split)
    core::split(
        &ctx.repo,
        &rs,
        0,
        &c1_2,
        Some("feature/auth-core-part1"),
        core::SplitOptions { no_ref: false },
    )
    .unwrap();

    // ASSERT (Split)
    let read = core::persistence::read_metadata(&ctx.repo, &s.id).unwrap();
    assert_eq!(read.steps.len(), 2);

    // ACT (Join)
    let rs = git_staircase::ResolvedSelector {
        staircase: core::resolve_by_id(&ctx.repo, &s.id).unwrap(),
        step_index: None,
    };
    core::join(
        &ctx.repo,
        &rs,
        0,
        1,
        core::JoinOptions {
            ref_action: core::JoinRefAction::Keep,
        },
    )
    .unwrap();

    // ASSERT (Join)
    let read = core::persistence::read_metadata(&ctx.repo, &s.id).unwrap();
    assert_eq!(read.steps.len(), 1);
    assert_eq!(read.steps[0].cut, c1_3);
}

#[test]
fn test_adopt_validation() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "feature/auth-ui"]);
    let _c2 = ctx.commit("file2.txt", "2", "commit 2");

    let s_empty = StaircaseMetadata {
        landing_policy: None,
        id: uuid::Uuid::new_v4().to_string(),
        name: "empty".to_string(),
        target: "main".to_string(),
        verification_policy: None,
        steps: vec![
            Step {
                id: String::new(),
                name: "step1".to_string(),
                cut: c1.clone(),
                branch: Some("feature/auth-core".to_string()),
            },
            Step {
                id: String::new(),
                name: "step2".to_string(),
                cut: c1.clone(),
                branch: Some("feature/auth-core".to_string()),
            },
        ],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT & ASSERT
    assert!(core::adopt(&ctx.repo, &s_empty).is_err());
}

#[test]
fn test_discover_forked() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "step1"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "step2a"]);
    let _c2a = ctx.commit("file2a.txt", "2a", "commit 2a");

    ctx.run_git(&["checkout", "step1"]);
    ctx.run_git(&["checkout", "-b", "step2b"]);
    let _c2b = ctx.commit("file2b.txt", "2b", "commit 2b");

    // ACT
    let discovered = core::discover(&ctx.repo, Some("main"), None, true).unwrap();

    // ASSERT
    assert_eq!(discovered.len(), 1);
    assert!(matches!(discovered[0], Discovery::Ambiguous(_)));
}

#[test]
fn test_verification_aggregate() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "feature/auth-ui"]);
    let _c2 = ctx.commit("file2.txt", "2", "commit 2");

    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();
    s.verification_policy = Some(VerificationPolicy {
        build_command: Some("true".to_string()),
        test_command: Some("true".to_string()),
        verify_each_prefix: false,
    });

    core::adopt(&ctx.repo, &s).unwrap();

    // ACT
    let results = core::verify(
        &ctx.repo,
        &ResolvedStaircase::Managed(s.clone()),
        None,
        None,
        Some(true),
        None,
    )
    .unwrap();

    // ASSERT
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
}

#[test]
fn test_verification_each_prefix() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "feature/auth-ui"]);
    let _c2 = ctx.commit("file2.txt", "2", "commit 2");

    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();
    s.verification_policy = Some(VerificationPolicy {
        build_command: Some("true".to_string()),
        test_command: Some("true".to_string()),
        verify_each_prefix: true,
    });

    core::adopt(&ctx.repo, &s).unwrap();

    // ACT
    let results = core::verify(
        &ctx.repo,
        &ResolvedStaircase::Managed(s.clone()),
        None,
        None,
        None,
        Some(true),
    )
    .unwrap();

    // ASSERT
    assert_eq!(results.len(), 2);
}

#[test]
fn test_split_implicit_staircase() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");
    let c1_2 = ctx.commit("file1_2.txt", "1.2", "commit 1.2");
    let _c1_3 = ctx.commit("file1_3.txt", "1.3", "commit 1.3");

    let discoveries = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let s = match &discoveries[0] {
        Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery"),
    };

    let rs = core::resolve_staircase(&ctx.repo, &s.id, None)
        .unwrap()
        .expect("Should find implicit staircase");
    assert!(!rs.is_managed());

    // ACT
    core::split(
        &ctx.repo,
        &rs,
        0,
        &c1_2,
        Some("feature/auth-core-part1"),
        core::SplitOptions { no_ref: false },
    )
    .expect("Split should succeed");

    // ASSERT
    let discoveries_after = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let s_after = match &discoveries_after[0] {
        Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery after split"),
    };

    let rs_after = core::resolve_staircase(&ctx.repo, &s_after.id, None)
        .unwrap()
        .expect("Should find staircase after split");
    assert!(!rs_after.is_managed());
    assert_eq!(rs_after.metadata().steps.len(), 2);
}

#[test]
fn test_id_lineage_remains_implicit() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    let _c1 = ctx.commit("file1.txt", "1", "commit 1");

    let discoveries = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let s = match &discoveries[0] {
        Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery"),
    };

    let rs = core::resolve_staircase(&ctx.repo, &s.id, None)
        .unwrap()
        .expect("Should find implicit staircase");

    // ACT
    use git_staircase::IdentityKind;
    let id = core::compute_identity(&ctx.repo, &rs, IdentityKind::Lineage).unwrap();

    // ASSERT
    assert!(!id.is_empty());
    assert!(id.starts_with("implicit@"));

    let rs_after = core::resolve_staircase(&ctx.repo, &id, None)
        .unwrap()
        .expect("Should find staircase");
    assert!(!rs_after.is_managed());
    assert_eq!(rs_after.metadata().id, id);
}

#[test]
fn test_slash_name_discovery() {
    // ARRANGE
    let ctx = TestContext::new();
    let metadata = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "feature/foo".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "s1".to_string(),
            cut: ctx.repo.resolve_commit("HEAD").unwrap(),
            branch: None,
        }],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    core::persistence::write_metadata(&ctx.repo, &metadata).unwrap();

    // ACT
    let list = core::persistence::list_staircases(&ctx.repo).unwrap();

    // ASSERT
    assert!(
        list.iter().any(|s| s.name == "feature/foo"),
        "Staircase with slash in name should be listed"
    );
}

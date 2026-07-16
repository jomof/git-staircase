use crate::common::TestContext;
use git_staircase::core;
use git_staircase::model::Discovery;

mod common;

#[test]
fn test_implicit_sequential_discovery() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feat-1"]);
    let _c1 = ctx.commit("1.txt", "1", "c1");
    ctx.run_git(&["checkout", "-b", "feat-2"]);
    let _c2 = ctx.commit("2.txt", "2", "c2");
    ctx.run_git(&["checkout", "-b", "feat"]);
    let _c3 = ctx.commit("3.txt", "3", "c3");

    let discoveries = core::discover(&ctx.repo, None, None, false).unwrap();
    assert_eq!(discoveries.len(), 1);

    if let Discovery::Linear(meta) = &discoveries[0] {
        assert_eq!(meta.primary_branch_layout.as_deref(), Some("sequential-v1"));
        assert_eq!(meta.branch_layout_base.as_deref(), Some("feat"));
        assert_eq!(meta.steps.len(), 3);
        assert_eq!(meta.steps[0].branch.as_deref(), Some("feat-1"));
        assert_eq!(meta.steps[1].branch.as_deref(), Some("feat-2"));
        assert_eq!(meta.steps[2].branch.as_deref(), Some("feat"));
    } else {
        panic!("Expected Linear discovery");
    }
}

#[test]
fn test_split_renumbers_and_adopts_implicit() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feat-1"]);
    let c1 = ctx.commit("1.txt", "1", "c1");
    ctx.run_git(&["checkout", "-b", "feat-2"]);
    let c2_mid = ctx.commit("2_mid.txt", "2_mid", "c2_mid");
    let c2 = ctx.commit("2.txt", "2", "c2");
    ctx.run_git(&["checkout", "-b", "feat"]);
    let c3 = ctx.commit("3.txt", "3", "c3");

    let rs = core::resolve_staircase(&ctx.repo, "feat", None)
        .unwrap()
        .unwrap()
        .staircase;
    assert!(matches!(rs, core::ResolvedStaircase::Implicit(_)));

    core::split(
        &ctx.repo,
        &rs,
        1,
        &c2_mid,
        Some("feat-2-split"),
        core::SplitOptions { no_ref: false },
    )
    .unwrap();

    let rs_after = core::resolve_staircase(&ctx.repo, "feat", None)
        .unwrap()
        .unwrap()
        .staircase;
    assert!(matches!(rs_after, core::ResolvedStaircase::Managed(_)));

    let meta = rs_after.metadata();
    assert_eq!(meta.steps.len(), 4);
    assert_eq!(meta.steps[0].branch.as_deref(), Some("feat-1"));
    assert_eq!(meta.steps[1].branch.as_deref(), Some("feat-2"));
    assert_eq!(meta.steps[2].branch.as_deref(), Some("feat-3"));
    assert_eq!(meta.steps[3].branch.as_deref(), Some("feat"));

    assert_eq!(ctx.run_git(&["rev-parse", "feat-1"]).trim(), c1);
    assert_eq!(ctx.run_git(&["rev-parse", "feat-2"]).trim(), c2_mid);
    assert_eq!(ctx.run_git(&["rev-parse", "feat-3"]).trim(), c2);
    assert_eq!(ctx.run_git(&["rev-parse", "feat"]).trim(), c3);
}

#[test]
fn test_reorder_renumbers_managed() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feat-1"]);
    let _c1 = ctx.commit("1.txt", "1", "c1");
    ctx.run_git(&["checkout", "-b", "feat-2"]);
    let _c2 = ctx.commit("2.txt", "2", "c2");
    ctx.run_git(&["checkout", "-b", "feat"]);
    let _c3 = ctx.commit("3.txt", "3", "c3");

    let rs = core::resolve_staircase(&ctx.repo, "feat", None)
        .unwrap()
        .unwrap()
        .staircase;
    let adopted = core::adopt(&ctx.repo, rs.metadata()).unwrap();
    let rs_managed = core::ResolvedStaircase::Managed(adopted);

    core::reorder(
        &ctx.repo,
        &rs_managed,
        &[1, 0, 2],
        core::ReorderOptions { no_restack: false },
    )
    .unwrap();

    let rs_after = core::resolve_staircase(&ctx.repo, "feat", None)
        .unwrap()
        .unwrap()
        .staircase;
    assert!(matches!(rs_after, core::ResolvedStaircase::Managed(_)));

    let meta = rs_after.metadata();
    assert_eq!(meta.steps.len(), 3);
    assert_eq!(meta.steps[0].branch.as_deref(), Some("feat-1"));
    assert_eq!(meta.steps[1].branch.as_deref(), Some("feat-2"));
    assert_eq!(meta.steps[2].branch.as_deref(), Some("feat"));

    let feat_1_oid = ctx.run_git(&["rev-parse", "feat-1"]).trim().to_string();
    let feat_2_oid = ctx.run_git(&["rev-parse", "feat-2"]).trim().to_string();
    let feat_oid = ctx.run_git(&["rev-parse", "feat"]).trim().to_string();

    let target_oid = ctx.run_git(&["rev-parse", "main"]).trim().to_string();
    assert!(ctx.repo.is_ancestor(&target_oid, &feat_1_oid).unwrap());
    assert!(ctx.repo.is_ancestor(&feat_1_oid, &feat_2_oid).unwrap());
    assert!(ctx.repo.is_ancestor(&feat_2_oid, &feat_oid).unwrap());

    assert_eq!(meta.steps[0].name, "feat-2");
    assert_eq!(meta.steps[1].name, "feat-1");
    assert_eq!(meta.steps[2].name, "feat");
}

#[test]
fn test_drop_renumbers_managed() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "feat-1"]);
    let c1 = ctx.commit("1.txt", "1", "c1");
    ctx.run_git(&["checkout", "-b", "feat-2"]);
    let _c2 = ctx.commit("2.txt", "2", "c2");
    ctx.run_git(&["checkout", "-b", "feat"]);
    let _c3 = ctx.commit("3.txt", "3", "c3");

    let rs = core::resolve_staircase(&ctx.repo, "feat", None)
        .unwrap()
        .unwrap()
        .staircase;
    let adopted = core::adopt(&ctx.repo, rs.metadata()).unwrap();
    let rs_managed = core::ResolvedStaircase::Managed(adopted);

    core::drop(
        &ctx.repo,
        &rs_managed,
        1,
        core::DropOptions {
            restack: true,
            leave_descendants_stale: false,
        },
    )
    .unwrap();

    let rs_after = core::resolve_staircase(&ctx.repo, "feat", None)
        .unwrap()
        .unwrap()
        .staircase;
    let meta = rs_after.metadata();
    assert_eq!(meta.steps.len(), 2);
    assert_eq!(meta.steps[0].branch.as_deref(), Some("feat-1"));
    assert_eq!(meta.steps[1].branch.as_deref(), Some("feat"));

    let branches = ctx.run_git(&["branch", "--list", "feat-2"]);
    assert!(branches.trim().is_empty(), "feat-2 should be deleted");

    let feat_1_oid = ctx.run_git(&["rev-parse", "feat-1"]).trim().to_string();
    let feat_oid = ctx.run_git(&["rev-parse", "feat"]).trim().to_string();

    assert_eq!(feat_1_oid, c1);
    assert!(ctx.repo.is_ancestor(&c1, &feat_oid).unwrap());
}

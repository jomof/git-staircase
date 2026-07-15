mod common;
use common::*;
use git_staircase::core::{self, ResolvedStaircase};
use git_staircase::model::{IdentityKind, StaircaseMetadata, Step};

#[test]
fn test_identity_lineage_and_nominal() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let staircase = StaircaseMetadata {
        landing_policy: None,
        id: "test-uuid".to_string(),
        name: "test-name".to_string(),
        target: target,
        steps: vec![],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT & ASSERT
    assert_eq!(
        core::compute_identity(
            &ctx.repo,
            &ResolvedStaircase::Managed(staircase.clone()),
            IdentityKind::Lineage
        )
        .unwrap(),
        "test-uuid"
    );
    assert_eq!(
        core::compute_identity(
            &ctx.repo,
            &ResolvedStaircase::Managed(staircase.clone()),
            IdentityKind::Nominal
        )
        .unwrap(),
        "test-name"
    );
}

#[test]
fn test_identity_revision() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let c1 = ctx.commit("f1.txt", "1", "c1");

    let s1 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![Step {
            id: String::new(),
            name: "s1".to_string(),
            cut: c1.clone(),
            branch: None,
        }],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT
    let id1 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s1.clone()),
        IdentityKind::Revision,
    )
    .unwrap();

    // ARRANGE (Modify)
    let c2 = ctx.commit("f2.txt", "2", "c2");
    let s2 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target,
        verification_policy: None,
        steps: vec![Step {
            id: String::new(),
            name: "s1".to_string(),
            cut: c2.clone(),
            branch: None,
        }],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT (New identity)
    let id2 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s2.clone()),
        IdentityKind::Revision,
    )
    .unwrap();

    // ASSERT
    assert_ne!(id1, id2);
}

#[test]
fn test_identity_body() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let c1 = ctx.commit("f1.txt", "1", "c1");
    let c2 = ctx.commit("f2.txt", "2", "c2");

    let s1 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: None,
            },
        ],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT
    let id1 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s1.clone()),
        IdentityKind::Body,
    )
    .unwrap();

    // ARRANGE (Join)
    let s2 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target,
        verification_policy: None,
        steps: vec![Step {
            id: String::new(),
            name: "s1+2".to_string(),
            cut: c2.clone(),
            branch: None,
        }],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT (New identity)
    let id2 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s2.clone()),
        IdentityKind::Body,
    )
    .unwrap();

    // ASSERT
    assert_eq!(id1, id2);
}

#[test]
fn test_identity_decomposition() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let c1 = ctx.commit("f1.txt", "1", "c1");
    let c2 = ctx.commit("f2.txt", "2", "c2");

    let s1 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: None,
            },
        ],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT
    let id1 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s1.clone()),
        IdentityKind::Decomposition,
    )
    .unwrap();

    // ARRANGE (Rebase)
    ctx.run_git(&["checkout", &target]);
    let c1_new = ctx.commit("f1.txt", "1", "c1 rebased");
    let c2_new = ctx.commit("f2.txt", "2", "c2 rebased");

    let s2 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1_new.clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2_new.clone(),
                branch: None,
            },
        ],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT (New identity)
    let id2 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s2.clone()),
        IdentityKind::Decomposition,
    )
    .unwrap();

    // ASSERT (Should be same after rebase with same patches)
    assert_eq!(id1, id2);

    // ARRANGE (Squash)
    let s3 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![Step {
            id: String::new(),
            name: "s1+2".to_string(),
            cut: c2_new.clone(),
            branch: None,
        }],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT (New identity)
    let id3 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s3.clone()),
        IdentityKind::Decomposition,
    )
    .unwrap();

    // ASSERT (Should be different after squash)
    assert_ne!(id2, id3);
}

#[test]
fn test_identity_outcome() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let c1 = ctx.commit("f1.txt", "1", "c1");
    let c2 = ctx.commit("f2.txt", "2", "c2");

    let s1 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: None,
            },
        ],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT
    let id1 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s1.clone()),
        IdentityKind::Outcome,
    )
    .unwrap();

    // ARRANGE (Reorder)
    ctx.run_git(&["checkout", "main"]);
    ctx.run_git(&["checkout", &target]);
    ctx.commit("f2.txt", "2", "c2 reordered");
    ctx.commit("f1.txt", "1", "c1 reordered");
    let top_new = ctx.run_git(&["rev-parse", "HEAD"]);

    let s2 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target,
        verification_policy: None,
        steps: vec![Step {
            id: String::new(),
            name: "reordered".to_string(),
            cut: top_new,
            branch: None,
        }],

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT (New identity)
    let id2 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s2.clone()),
        IdentityKind::Outcome,
    )
    .unwrap();

    // ASSERT (Same final tree)
    assert_eq!(id1, id2);
}

#[test]
fn test_revision_identity_prefix() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();
    let staircase = StaircaseMetadata {
        landing_policy: None,
        id: "test-uuid".to_string(),
        name: "test-name".to_string(),
        target: target,
        steps: vec![],
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT
    let revision_id = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(staircase),
        IdentityKind::Revision,
    )
    .unwrap();

    // ASSERT
    // The revision ID should start with "sha1:" or "sha256:"
    assert!(
        revision_id.starts_with("sha1:") || revision_id.starts_with("sha256:"),
        "Revision ID '{}' must start with an algorithm prefix (e.g., 'sha1:')",
        revision_id
    );
}

#[test]
fn test_identity_review() {
    // ARRANGE
    let ctx = TestContext::new();
    let target = ctx.repo.resolve_commit("main").unwrap();

    // Commit with Change-Id
    let c1 = ctx.commit(
        "f1.txt",
        "1",
        "c1\n\nChange-Id: I1234567890abcdef1234567890abcdef12345678",
    );
    let c2 = ctx.commit(
        "f2.txt",
        "2",
        "c2\n\nChange-Id: Iabcdef1234567890abcdef1234567890abcdef12",
    );

    let s1 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: None,
            },
        ],
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT
    let id1 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s1.clone()),
        IdentityKind::Review,
    )
    .unwrap();

    // ARRANGE (Rebase - preserves Change-Ids but changes OIDs)
    ctx.run_git(&["checkout", "main"]);
    let c1_new = ctx.commit(
        "f1.txt",
        "1",
        "c1 rebased\n\nChange-Id: I1234567890abcdef1234567890abcdef12345678",
    );
    let c2_new = ctx.commit(
        "f2.txt",
        "2",
        "c2 rebased\n\nChange-Id: Iabcdef1234567890abcdef1234567890abcdef12",
    );

    let s2 = StaircaseMetadata {
        landing_policy: None,
        id: "uuid".to_string(),
        name: "name".to_string(),
        target: target.clone(),
        verification_policy: None,
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1_new.clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2_new.clone(),
                branch: None,
            },
        ],
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // ACT (New identity)
    let id2 = core::compute_identity(
        &ctx.repo,
        &ResolvedStaircase::Managed(s2.clone()),
        IdentityKind::Review,
    )
    .unwrap();

    // ASSERT
    assert!(!id1.is_empty(), "Review identity should not be empty");
    assert_eq!(
        id1, id2,
        "Review identity should be stable across rebase if Change-Ids are preserved"
    );
}


use crate::common::*;
use git_staircase::core::{self, ArchiveOptions, UnarchiveBranchesMode, UnarchiveOptions};
use git_staircase::model::{LifecycleState, StaircaseLink, StepMetadata};

#[test]
fn test_user_metadata_lifecycle() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature-a"]);
    commit(dir, "a.txt", "a", "commit a");
    run_git(dir, &["checkout", "-b", "feature-b"]);
    commit(dir, "b.txt", "b", "commit b");

    let discovered = core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(mut sc) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    sc.name = "my-feature".to_string();

    let sc = core::adopt(&repo, &sc).unwrap();
    let selector = resolved_selector_from(&sc);

    // Set Title & Description
    let record = core::set_title(&repo, &selector, "My Feature Staircase").unwrap();
    assert_eq!(
        record.user_metadata.title.as_deref(),
        Some("My Feature Staircase")
    );

    let record =
        core::set_description(&repo, &selector, "Detailed description of feature").unwrap();
    assert_eq!(
        record.user_metadata.description.as_deref(),
        Some("Detailed description of feature")
    );

    // Add & Remove Labels
    let record = core::add_label(&repo, &selector, "frontend").unwrap();
    assert!(
        record
            .user_metadata
            .labels
            .contains(&"frontend".to_string())
    );

    let record = core::add_label(&repo, &selector, "urgent").unwrap();
    assert_eq!(record.user_metadata.labels.len(), 2);

    let record = core::remove_label(&repo, &selector, "frontend").unwrap();
    assert!(
        !record
            .user_metadata
            .labels
            .contains(&"frontend".to_string())
    );

    // Add Link
    let link = StaircaseLink {
        id: "link-1".to_string(),
        relationship: "tracker".to_string(),
        url: "https://b.corp.google.com/123456".to_string(),
        label: Some("Bug 123456".to_string()),
        description: None,
    };
    let record = core::add_link(&repo, &selector, link).unwrap();
    assert_eq!(record.user_metadata.links.len(), 1);

    // Step Metadata
    let step_meta = StepMetadata {
        title: Some("DB Step".to_string()),
        description: Some("Initial database migration".to_string()),
        labels: vec!["db-schema".to_string()],
        links: vec![],
    };
    let _record = core::update_step_metadata(&repo, &selector, "feature-a", step_meta).unwrap();
    let fetched_step_meta = core::get_step_metadata(&repo, &selector, "feature-a").unwrap();
    assert_eq!(
        fetched_step_meta.description.as_deref(),
        Some("Initial database migration")
    );
}

#[test]
fn test_archive_and_mutation_guard() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "step-1"]);
    commit(dir, "s1.txt", "s1", "step 1");
    run_git(dir, &["checkout", "-b", "step-2"]);
    commit(dir, "s2.txt", "s2", "step 2");

    let discovered = core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(mut sc) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    sc.name = "archival-staircase".to_string();

    let sc = core::adopt(&repo, &sc).unwrap();
    let selector = resolved_selector_from(&sc);

    // Add branch config
    let _ = repo.run(&[
        "config",
        "branch.step-1.description",
        "Step 1 branch description",
    ]);

    // Archive staircase
    let archive_opts = ArchiveOptions {
        reason: Some("Completed feature".to_string()),
        dry_run: false,
        snapshot_drafts: false,
        detach_dirty_worktrees: false,
        leave_worktrees: false,
    };

    let res = core::archive_staircase(&repo, &selector, &archive_opts).unwrap();
    assert_eq!(res.canonical_name, "archival-staircase");

    // Re-resolve archived selector to load updated lifecycle state
    let archived_sel = core::resolve_staircase(&repo, "archival-staircase", None)
        .unwrap()
        .unwrap();

    // Check branch refs removed from refs/heads/
    assert!(repo.resolve_ref("refs/heads/step-1").is_err());
    assert!(repo.resolve_ref("refs/heads/step-2").is_err());

    // Check branch config removed
    assert!(
        repo.run(&["config", "--get", "branch.step-1.description"])
            .is_err()
    );

    // Check archived record ref exists
    let archive_record_ref = format!("refs/staircase-archive/{}/record", sc.id);
    let record = core::persistence::read_record(&repo, &archive_record_ref).unwrap();
    assert_eq!(record.lifecycle.state, LifecycleState::Archived);
    assert!(record.archive_manifest.is_some());

    // Structural mutation guard tests
    let split_res = core::split(
        &repo,
        &archived_sel.staircase,
        0,
        &sc.steps[0].cut,
        Some("step-1-part2"),
        core::SplitOptions { no_ref: false },
    );
    assert!(split_res.is_err());
    assert!(
        split_res
            .unwrap_err()
            .to_string()
            .contains("staircase is archived")
    );

    let join_res = core::join(
        &repo,
        &archived_sel.staircase,
        0,
        1,
        core::JoinOptions {
            ref_action: core::JoinRefAction::Keep,
        },
    );
    assert!(join_res.is_err());
    assert!(
        join_res
            .unwrap_err()
            .to_string()
            .contains("staircase is archived")
    );
}

#[test]
fn test_unarchive_lifecycle() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "b1"]);
    commit(dir, "1.txt", "1", "c1");
    run_git(dir, &["checkout", "-b", "b2"]);
    commit(dir, "2.txt", "2", "c2");

    let discovered = core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(mut sc) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    sc.name = "unarchive-test".to_string();

    let sc = core::adopt(&repo, &sc).unwrap();
    let selector = resolved_selector_from(&sc);

    // Add branch config
    let _ = repo.run(&["config", "branch.b1.description", "Branch 1 description"]);

    // Archive
    let archive_opts = ArchiveOptions {
        reason: None,
        dry_run: false,
        snapshot_drafts: false,
        detach_dirty_worktrees: false,
        leave_worktrees: false,
    };
    core::archive_staircase(&repo, &selector, &archive_opts).unwrap();

    // Unarchive
    let unarchive_opts = UnarchiveOptions {
        new_name: None,
        branch_base: None,
        branches_mode: UnarchiveBranchesMode::Exact,
        adopt_existing_branches: false,
        reattach_worktrees: false,
    };

    let unarch_res = core::unarchive_staircase(&repo, &selector, &unarchive_opts).unwrap();
    assert_eq!(unarch_res.canonical_name, "unarchive-test");
    assert_eq!(unarch_res.restored_branches, vec!["b1", "b2"]);

    // Check branch refs restored
    assert!(repo.resolve_ref("refs/heads/b1").is_ok());
    assert!(repo.resolve_ref("refs/heads/b2").is_ok());

    // Check branch config restored
    let cfg = repo
        .run(&["config", "--get", "branch.b1.description"])
        .unwrap();
    assert_eq!(cfg.trim(), "Branch 1 description");

    // Check active record ref restored
    let active_record =
        core::persistence::read_record(&repo, &format!("refs/staircases/{}", sc.name)).unwrap();
    assert_eq!(active_record.lifecycle.state, LifecycleState::Active);

    // Check archive refs removed
    let archive_refs = repo
        .run(&[
            "for-each-ref",
            &format!("refs/staircase-archive/{}/", sc.id),
        ])
        .unwrap();
    assert!(archive_refs.trim().is_empty());
}

#[test]
fn test_release_canonical_name() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feat-1"]);
    commit(dir, "f.txt", "1", "commit");

    let discovered = core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(mut sc) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    sc.name = "reserved-name".to_string();

    let sc = core::adopt(&repo, &sc).unwrap();
    let selector = resolved_selector_from(&sc);

    // Archive
    let archive_opts = ArchiveOptions {
        reason: None,
        dry_run: false,
        snapshot_drafts: false,
        detach_dirty_worktrees: false,
        leave_worktrees: false,
    };
    core::archive_staircase(&repo, &selector, &archive_opts).unwrap();

    // Release name
    core::release_staircase_name(&repo, &selector).unwrap();

    // Now another staircase can be adopted with "reserved-name"
    run_git(dir, &["checkout", "main"]);
    run_git(dir, &["checkout", "-b", "feat-2"]);
    commit(dir, "g.txt", "2", "commit 2");

    let discovered2 = core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(mut sc2) = discovered2[0].clone() else {
        panic!("Expected linear discovery");
    };
    sc2.name = "reserved-name".to_string();

    let adopted2 = core::adopt(&repo, &sc2);
    assert!(adopted2.is_ok());
}

fn resolved_selector_from(meta: &git_staircase::StaircaseMetadata) -> core::ResolvedSelector {
    core::ResolvedSelector {
        staircase: core::ResolvedStaircase::Managed(meta.clone()),
        step_index: None,
    }
}

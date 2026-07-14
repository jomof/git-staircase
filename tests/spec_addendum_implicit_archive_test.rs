mod common;
use common::*;
use git_staircase::core::refs::StaircaseRefs;
use git_staircase::core::{self, ArchiveOptions, UnarchiveBranchesMode, UnarchiveOptions};

#[test]
fn test_direct_implicit_archive_without_adoption() {
    let (_tmp, repo) = setup_repo();

    repo.run(&["checkout", "-b", "dry-move-1"]).unwrap();
    commit(&repo.workdir, "1.txt", "1", "Commit 1");
    repo.run(&["checkout", "-b", "dry-move-2"]).unwrap();
    commit(&repo.workdir, "2.txt", "2", "Commit 2");

    let discovered = core::discover(&repo, Some("main"), None, false).unwrap();
    assert_eq!(discovered.len(), 1, "Expected 1 discovered implicit staircase");

    let sel = core::resolve_staircase(&repo, "dry-move-2", None)
        .unwrap()
        .expect("Failed to resolve selector for dry-move-2");

    assert!(
        !sel.is_managed(),
        "Expected resolved staircase to be implicit before archive"
    );

    let archive_opts = ArchiveOptions {
        reason: Some("Archiving implicit directly".to_string()),
        adopt: false,
        no_adopt: true,
        ..Default::default()
    };

    let result = core::archive_staircase(&repo, &sel, &archive_opts)
        .expect("Direct implicit archive failed");

    assert_eq!(result.source_representation, "implicit");
    assert_eq!(result.archive_kind, "implicit-snapshot");
    assert!(!result.adopted, "Expected adopted flag to be false");
    assert_eq!(result.lineage_id, None, "Expected lineage_id to be None");
    assert!(
        result.archive_id.is_some(),
        "Expected allocated archive_id for implicit snapshot"
    );

    let aid = result.archive_id.unwrap();
    let implicit_record_ref = StaircaseRefs::implicit_archive_record(&aid);
    assert!(
        repo.resolve_ref_opt(&implicit_record_ref)
            .unwrap()
            .is_some(),
        "Implicit archive record ref should exist at {}",
        implicit_record_ref
    );

    assert!(
        repo.resolve_ref_opt("refs/heads/dry-move-1")
            .unwrap()
            .is_none(),
        "Active branch dry-move-1 should be removed after implicit archive"
    );
    assert!(
        repo.resolve_ref_opt("refs/heads/dry-move-2")
            .unwrap()
            .is_none(),
        "Active branch dry-move-2 should be removed after implicit archive"
    );

    // Verify active discovery suppression
    let active_list = core::list(&repo, Default::default()).unwrap();
    assert!(
        active_list.is_empty(),
        "Archived implicit staircase should be suppressed from active list"
    );

    // Verify list --archived includes the snapshot
    let filter_archived = core::ListFilter {
        archived: true,
        ..Default::default()
    };
    let archived_list = core::list(&repo, filter_archived).unwrap();
    assert_eq!(
        archived_list.len(),
        1,
        "Expected 1 entry when listing archived staircases"
    );
}

#[test]
fn test_explicit_adopt_before_archive() {
    let (_tmp, repo) = setup_repo();

    repo.run(&["checkout", "-b", "feat-a"]).unwrap();
    commit(&repo.workdir, "a.txt", "a", "Commit A");
    repo.run(&["checkout", "-b", "feat-b"]).unwrap();
    commit(&repo.workdir, "b.txt", "b", "Commit B");

    let sel = core::resolve_staircase(&repo, "feat-b", None)
        .unwrap()
        .expect("Failed to resolve selector for feat-b");

    let archive_opts = ArchiveOptions {
        reason: Some("Adopting before archive".to_string()),
        adopt: true,
        ..Default::default()
    };

    let result = core::archive_staircase(&repo, &sel, &archive_opts)
        .expect("Explicit adopt archive failed");

    assert_eq!(result.source_representation, "implicit");
    assert_eq!(result.archive_kind, "managed-lineage");
    assert!(result.adopted, "Expected adopted flag to be true");
    assert!(
        result.lineage_id.is_some(),
        "Expected lineage_id for adopted managed archive"
    );
}

#[test]
fn test_unarchive_direct_implicit_snapshot() {
    let (_tmp, repo) = setup_repo();

    repo.run(&["checkout", "-b", "restore-1"]).unwrap();
    commit(&repo.workdir, "r1.txt", "r1", "Commit R1");
    repo.run(&["checkout", "-b", "restore-2"]).unwrap();
    commit(&repo.workdir, "r2.txt", "r2", "Commit R2");

    let sel = core::resolve_staircase(&repo, "restore-2", None)
        .unwrap()
        .expect("Failed to resolve selector");

    let archive_opts = ArchiveOptions {
        adopt: false,
        no_adopt: true,
        ..Default::default()
    };
    let arch_res = core::archive_staircase(&repo, &sel, &archive_opts).unwrap();
    let aid = arch_res.archive_id.unwrap();

    let arch_sel = core::resolve_staircase(&repo, &format!("archive@{}", aid), None)
        .unwrap()
        .expect("Failed to resolve implicit archive selector by archive ID");

    let unarchive_opts = UnarchiveOptions {
        branches_mode: UnarchiveBranchesMode::Exact,
        ..Default::default()
    };

    let unarch_res = core::unarchive_staircase(&repo, &arch_sel, &unarchive_opts)
        .expect("Unarchiving implicit snapshot failed");

    assert_eq!(unarch_res.restored_branches, vec!["restore-1", "restore-2"]);

    assert!(
        repo.resolve_ref_opt("refs/heads/restore-1")
            .unwrap()
            .is_some(),
        "Branch restore-1 should be restored"
    );
    assert!(
        repo.resolve_ref_opt("refs/heads/restore-2")
            .unwrap()
            .is_some(),
        "Branch restore-2 should be restored"
    );

    let implicit_record_ref = StaircaseRefs::implicit_archive_record(&aid);
    assert!(
        repo.resolve_ref_opt(&implicit_record_ref)
            .unwrap()
            .is_none(),
        "Implicit archive record ref should be deleted after unarchive"
    );
}

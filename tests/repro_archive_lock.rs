mod common;
use common::*;
use git_staircase::core;
use git_staircase::core::archive::ArchiveOptions;
use git_staircase::core::persistence;
use git_staircase::core::refs::StaircaseRefs;
use git_staircase::core::resolution::resolve_staircase;
use git_staircase::model::LifecycleState;
use std::fs;

#[test]
fn test_archive_leaves_stale_lock_on_failure() {
    // ARRANGE
    let ctx = TestContext::new();

    // Create a staircase
    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    ctx.commit("file1.txt", "1", "commit 1");
    ctx.run_git(&["checkout", "-b", "feature/auth-ui"]);
    ctx.commit("file2.txt", "2", "commit 2");

    let discovered = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let s = core::adopt(&ctx.repo, &s).unwrap();
    let id = s.id.clone();

    // Verify it's active
    let record = persistence::read_record(
        &ctx.repo,
        &StaircaseRefs::record(&id, LifecycleState::Active),
    )
    .unwrap();
    assert_eq!(record.lifecycle.state, LifecycleState::Active);

    let selector = resolve_staircase(&ctx.repo, &s.name, None)
        .unwrap()
        .expect("Staircase not found");

    // Now delete the branch to induce failure during archive
    ctx.run_git(&["branch", "-D", "feature/auth-core"]);

    let options = ArchiveOptions::default();

    // ACT
    let result = core::archive_staircase(&ctx.repo, &selector, &options);

    // ASSERT failure
    assert!(
        result.is_err(),
        "Archive should have failed because a branch was missing"
    );

    // CHECK for stale locks
    let locks_dir = ctx
        .repo
        .common_dir()
        .unwrap()
        .join("staircase")
        .join("locks");
    if locks_dir.exists() {
        let entries: Vec<_> = fs::read_dir(locks_dir).unwrap().collect();
        assert!(
            entries.is_empty(),
            "Locks directory should be empty after failed archive, but found: {:?}",
            entries
        );
    }
}

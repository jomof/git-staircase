use git_staircase::GitRepo;
use git_staircase::core::ResolvedStaircase;
use git_staircase::core::manipulation::{RebaseOptions, restack};
use git_staircase::git::TreeEntry;
use tempfile::tempdir;

#[test]
fn test_restack_manual_rebase_of_multiple_steps() {
    let dir = tempdir().unwrap();
    let repo_dir = dir.path();

    std::process::Command::new("git")
        .arg("init")
        .current_dir(repo_dir)
        .output()
        .unwrap();

    let repo = GitRepo::new(repo_dir.to_path_buf());

    // 1. Create a staircase with two steps
    let blob_oid = repo.write_blob("content").unwrap();
    let tree_oid = repo
        .write_tree(&[TreeEntry::blob(blob_oid.clone(), "file")])
        .unwrap();
    let root_commit = repo
        .command()
        .args(&["commit-tree", &tree_oid, "-m", "root"])
        .run()
        .unwrap();

    // Step 1 commit
    let c1 = repo
        .command()
        .args(&["commit-tree", &tree_oid, "-p", &root_commit, "-m", "c1"])
        .run()
        .unwrap();

    // Step 2 commit
    let c2 = repo
        .command()
        .args(&["commit-tree", &tree_oid, "-p", &c1, "-m", "c2"])
        .run()
        .unwrap();

    // 2. Set up the staircase metadata
    let metadata = git_staircase::model::StaircaseMetadata {
        id: "test-staircase".to_string(),
        name: "test".to_string(),
        target: root_commit.clone(),
        steps: vec![
            git_staircase::model::Step {
                id: "s1".to_string(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("b1".to_string()),
            },
            git_staircase::model::Step {
                id: "s2".to_string(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("b2".to_string()),
            },
        ],
        landing_policy: None,
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // 3. Mock the staircase as resolved (Implicit)
    let staircase = ResolvedStaircase::Implicit(metadata);

    // 4. Update the branches to point to different commits (simulating local manual rebase)
    let a1 = repo
        .command()
        .args(&["commit-tree", &tree_oid, "-p", &root_commit, "-m", "a1"])
        .run()
        .unwrap();
    let a2 = repo
        .command()
        .args(&["commit-tree", &tree_oid, "-p", &a1, "-m", "a2"])
        .run()
        .unwrap();

    repo.update_branch("b1", &a1).unwrap();
    repo.update_branch("b2", &a2).unwrap();

    // 5. Run restack - this should succeed with my fix
    restack(
        &repo,
        &staircase,
        RebaseOptions {
            leave_upper_steps_stale: false,
        },
    )
    .expect("Restack should succeed after fix");

    // 6. Verify that the staircase is now updated and clean
    let id = staircase.metadata().id.clone();
    let latest_metadata = git_staircase::core::persistence::read_metadata(&repo, &id)
        .expect("Metadata should be readable");

    assert_eq!(
        latest_metadata.steps[0].cut, a1,
        "Step 1 cut should be updated to a1"
    );
    assert_eq!(
        latest_metadata.steps[1].cut, a2,
        "Step 2 cut should be updated to a2"
    );

    let status =
        git_staircase::core::status::get_status_metadata(&repo, latest_metadata, true, false)
            .unwrap();
    assert!(status.is_clean, "Staircase should be clean after restack");
}

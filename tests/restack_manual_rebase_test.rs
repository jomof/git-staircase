use git_staircase::GitRepo;
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
    let b_root = repo.write_blob("root").unwrap();
    let tree_root = repo
        .write_tree(&[TreeEntry::blob(b_root, "root.txt")])
        .unwrap();
    let root_commit = repo
        .command()
        .args(&["commit-tree", &tree_root, "-m", "root"])
        .run()
        .unwrap();
    repo.update_branch("main", &root_commit).unwrap();
    repo.command().args(&["checkout", "main"]).run().unwrap();

    let b1 = repo.write_blob("c1").unwrap();
    let tree_c1 = repo
        .write_tree(&[
            TreeEntry::blob(repo.write_blob("root").unwrap(), "root.txt"),
            TreeEntry::blob(b1, "file1.txt"),
        ])
        .unwrap();
    let c1 = repo
        .command()
        .args(&["commit-tree", &tree_c1, "-p", &root_commit, "-m", "c1"])
        .run()
        .unwrap();

    let b2 = repo.write_blob("c2").unwrap();
    let tree_c2 = repo
        .write_tree(&[
            TreeEntry::blob(repo.write_blob("root").unwrap(), "root.txt"),
            TreeEntry::blob(repo.write_blob("c1").unwrap(), "file1.txt"),
            TreeEntry::blob(b2, "file2.txt"),
        ])
        .unwrap();
    let c2 = repo
        .command()
        .args(&["commit-tree", &tree_c2, "-p", &c1, "-m", "c2"])
        .run()
        .unwrap();

    repo.update_branch("b1", &c1).unwrap();
    repo.update_branch("b2", &c2).unwrap();

    // 2. Set up the staircase metadata
    let metadata = git_staircase::model::StaircaseMetadata {
        id: "implicit@test".to_string(),
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

    git_staircase::core::persistence::write_metadata(&repo, &metadata).unwrap();
    let staircase = git_staircase::core::resolution::resolve_by_name(&repo, "test").unwrap();

    // 4. Update branch b1 to point to a1 (simulating local manual rebase)
    let b_a1 = repo.write_blob("a1").unwrap();
    let tree_a1 = repo
        .write_tree(&[
            TreeEntry::blob(repo.write_blob("root").unwrap(), "root.txt"),
            TreeEntry::blob(b_a1, "file1.txt"),
        ])
        .unwrap();
    let a1 = repo
        .command()
        .args(&["commit-tree", &tree_a1, "-p", &root_commit, "-m", "a1"])
        .run()
        .unwrap();
    repo.update_branch("b1", &a1).unwrap();

    // 5. Run restack
    restack(
        &repo,
        &staircase,
        RebaseOptions {
            leave_upper_steps_stale: false,
        },
    )
    .expect("Restack should succeed after fix");

    // 6. Verify that the staircase is now updated and clean
    let latest_metadata = git_staircase::core::persistence::read_metadata(&repo, "test")
        .expect("Metadata should be readable");

    assert_eq!(
        latest_metadata.steps[0].cut, a1,
        "Step 1 cut should be updated to a1"
    );
    let step2_parent = repo
        .run(&["rev-parse", &format!("{}^", latest_metadata.steps[1].cut)])
        .unwrap();
    assert_eq!(step2_parent, a1, "Step 2 cut parent should be a1");

    let status =
        git_staircase::core::status::get_status_metadata(&repo, latest_metadata, true).unwrap();
    assert!(status.is_clean, "Staircase should be clean after restack");
}

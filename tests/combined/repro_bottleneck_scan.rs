use crate::common::run_git;
use git_staircase::GitRepo;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_bottleneck_worktree_scan() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    run_git(repo_path, &["init"]);

    // Create commit 1
    fs::write(repo_path.join("file"), "A").unwrap();
    run_git(repo_path, &["add", "file"]);
    run_git(repo_path, &["commit", "-m", "A"]);

    // Create commit 2
    fs::write(repo_path.join("file"), "B").unwrap();
    run_git(repo_path, &["add", "file"]);
    run_git(repo_path, &["commit", "-m", "B"]);

    let oid = run_git(repo_path, &["rev-parse", "HEAD"]);

    // Adopt a staircase
    let repo = GitRepo::new(repo_path.to_path_buf());
    let meta = git_staircase::model::StaircaseMetadata {
        id: uuid::Uuid::new_v4().to_string(),
        name: "test-staircase".to_string(),
        target: "HEAD^".to_string(),
        steps: vec![git_staircase::model::Step {
            id: uuid::Uuid::new_v4().to_string(),
            name: "step1".to_string(),
            cut: oid.clone(),
            branch: None,
        }],
        landing_policy: None,
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    git_staircase::core::persistence::write_metadata(&repo, &meta).unwrap();
    let record = git_staircase::core::persistence::read_record(
        &repo,
        &git_staircase::core::refs::StaircaseRefs::state_record(&meta.id),
    )
    .unwrap();
    let selector = git_staircase::core::ResolvedSelector {
        staircase: git_staircase::core::ResolvedStaircase::Managed(record.metadata),
        step_index: None,
    };

    // MEASURE 1: Normal speed
    let start = Instant::now();
    git_staircase::core::local::name_staircase(&repo, &selector, "new-name-1", false).unwrap();
    let duration_normal = start.elapsed();
    println!("Duration with few files: {:?}", duration_normal);

    // Add many files to the worktree to slow down git write-tree / diff
    for i in 0..2000 {
        fs::write(repo_path.join(format!("file-{}", i)), "content").unwrap();
    }

    // Refresh selector
    let record = git_staircase::core::persistence::read_record(
        &repo,
        &git_staircase::core::refs::StaircaseRefs::state_record(&meta.id),
    )
    .unwrap();
    let selector = git_staircase::core::ResolvedSelector {
        staircase: git_staircase::core::ResolvedStaircase::Managed(record.metadata),
        step_index: None,
    };

    // MEASURE 2: Speed with many untracked files
    let start = Instant::now();
    git_staircase::core::local::name_staircase(&repo, &selector, "new-name-2", false).unwrap();
    let duration_many_files = start.elapsed();
    println!("Duration with 2000 files: {:?}", duration_many_files);
}

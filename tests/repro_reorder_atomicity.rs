mod common;
use common::*;
use git_staircase::core;
use git_staircase::model::{ResolvedStaircase, StaircaseMetadata, Step};

#[test]
fn test_reorder_partial_failure_leaves_desync() -> anyhow::Result<()> {
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;
    let target = run_git(repo_path, &["rev-parse", "HEAD"]);

    run_git(repo_path, &["checkout", "-b", "step1", &target]);
    let c1 = commit(repo_path, "f1.txt", "1", "c1");

    run_git(repo_path, &["checkout", "-b", "step2", &c1]);
    let c2 = commit(repo_path, "conflict.txt", "2", "c2");

    let metadata = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "step1".to_string(),
                cut: c1.clone(),
                branch: Some("step1".to_string()),
            },
            Step {
                name: "step2".to_string(),
                cut: c2.clone(),
                branch: Some("step2".to_string()),
            },
        ],
        verification_policy: None,
    };
    core::adopt(&repo, &metadata).unwrap();

    run_git(repo_path, &["checkout", "main"]);
    commit(repo_path, "conflict.txt", "conflict", "conflict on main");

    // Reorder [0, 1]. Step 1 rebases onto new main (success), Step 2 rebases onto new step 1 (fails).
    let rs = ResolvedStaircase::Managed(metadata);
    let _ = core::reorder(&repo, &rs, &[0, 1]);

    let saved_metadata = repo.read_metadata("test-id").unwrap();
    let actual_c1 = run_git(repo_path, &["rev-parse", "step1"]);
    assert_eq!(saved_metadata.steps[0].cut, actual_c1);
    Ok(())
}

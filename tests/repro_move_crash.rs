mod common;
use common::*;
use git_staircase::core;
use git_staircase::model::{ResolvedStaircase, StaircaseMetadata, Step};

#[test]
fn test_move_commits_empty_panic() -> anyhow::Result<()> {
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;
    let target_oid = run_git(repo_path, &["rev-parse", "HEAD"]);

    let metadata = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test-staircase".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "s1".to_string(),
                cut: target_oid.clone(),
                branch: None,
            },
            Step {
                name: "s2".to_string(),
                cut: target_oid.clone(),
                branch: None,
            },
        ],
        verification_policy: None,
    };

    let rs = ResolvedStaircase::Managed(metadata);

    let _ = core::move_commits(&repo, &rs, 1, 0, &[]);
    Ok(())
}

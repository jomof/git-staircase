mod common;
use common::*;
use git_staircase::*;
use std::fs;

#[test]
fn test_verify_leaves_detached_head() {
    let (_tmp, repo) = setup_repo();
    let repo_dir = &repo.workdir;

    run_git(repo_dir, &["checkout", "-b", "s1"]);
    fs::write(repo_dir.join("s1.txt"), "s1").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "s1 commit"]);
    let c1 = run_git(repo_dir, &["rev-parse", "HEAD"]);

    run_git(repo_dir, &["checkout", "-b", "s2"]);
    fs::write(repo_dir.join("s2.txt"), "s2").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "s2 commit"]);
    let c2 = run_git(repo_dir, &["rev-parse", "HEAD"]);

    let sc = StaircaseMetadata {
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: Some(VerificationPolicy {
            build_command: Some("true".to_string()),
            test_command: None,
            verify_each_prefix: true,
        }),
    };
    git_staircase::core::adopt(&repo, &sc).unwrap();

    run_git(repo_dir, &["checkout", "main"]);

    // Force a checkout failure on s2 by creating an untracked file that conflicts
    fs::write(repo_dir.join("s2.txt"), "blocker").unwrap();

    let res = git_staircase::core::verify(
        &repo,
        &ResolvedStaircase::Managed(sc),
        None,
        None,
        None,
        None,
    );
    assert!(res.is_err());

    let current = run_git(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(
        current, "main",
        "Branch was NOT restored after checkout failure at step 2!"
    );
}

mod common;
use common::*;
use git_staircase::*;
use std::fs;

#[test]
fn test_restack_propagation() {
    let (_tmp, repo) = setup_repo();
    let repo_dir = &repo.workdir;

    // 1. Repo already initialized with one commit by setup_repo()

    // 2. Create a chain of 3 branches: s1 -> s2 -> s3
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

    run_git(repo_dir, &["checkout", "-b", "s3"]);
    fs::write(repo_dir.join("s3.txt"), "s3").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "s3 commit"]);
    let c3 = run_git(repo_dir, &["rev-parse", "HEAD"]);

    // 3. Adopt as a staircase
    let sc = StaircaseMetadata {
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
            Step {
                name: "s3".to_string(),
                cut: c3.clone(),
                branch: Some("s3".to_string()),
            },
        ],
        verification_policy: None,
    };
    git_staircase::core::adopt(&repo, &sc).unwrap();

    // 4. Modify s1 by rebasing it onto a new commit on main
    run_git(repo_dir, &["checkout", "main"]);
    fs::write(repo_dir.join("main2.txt"), "main2").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "main update"]);

    run_git(repo_dir, &["checkout", "s1"]);
    run_git(repo_dir, &["rebase", "main"]);
    let s1_new = run_git(repo_dir, &["rev-parse", "HEAD"]);

    let rs = git_staircase::core::resolve_staircase(&repo, "test", None)
        .unwrap()
        .unwrap();

    // 5. Run restack
    git_staircase::core::restack(&repo, &rs).unwrap();

    // 6. Verify results
    let s1_final = run_git(repo_dir, &["rev-parse", "s1"]);
    let s2_final = run_git(repo_dir, &["rev-parse", "s2"]);
    let s3_final = run_git(repo_dir, &["rev-parse", "s3"]);

    assert_eq!(s1_final, s1_new);

    // s2 should have been rebased onto s1_new
    assert!(repo.is_ancestor(&s1_final, &s2_final).unwrap());

    // s3 SHOULD have been rebased onto s2_new
    assert!(
        repo.is_ancestor(&s2_final, &s3_final).unwrap(),
        "s3 was not rebased onto s2"
    );
}

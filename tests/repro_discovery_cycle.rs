mod common;
use common::*;
use git_staircase::core::discover;
use std::fs;

#[test]
fn test_discovery_with_duplicate_oids() {
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    // Create a commit on main
    fs::write(repo_path.join("file_new"), "content").unwrap();
    run_git(repo_path, &["add", "file_new"]);
    run_git(repo_path, &["commit", "-m", "base commit"]);

    // Create two branches at the same OID, ahead of main
    run_git(repo_path, &["checkout", "-b", "branch-a"]);
    fs::write(repo_path.join("file_new"), "modified").unwrap();
    run_git(repo_path, &["add", "file_new"]);
    run_git(repo_path, &["commit", "-m", "work"]);

    run_git(repo_path, &["branch", "branch-b", "branch-a"]);

    let onto = if repo.resolve_ref("master").is_ok() {
        "master"
    } else {
        "main"
    };
    let discoveries = discover(&repo, Some(onto)).unwrap();

    let found = discoveries.iter().any(|d| match d {
        git_staircase::model::Discovery::Linear(m) => m
            .steps
            .iter()
            .any(|s| s.name == "branch-a" || s.name == "branch-b"),
        git_staircase::model::Discovery::Ambiguous(f) => {
            f.steps.contains_key("branch-a") || f.steps.contains_key("branch-b")
        }
    });

    assert!(
        found,
        "Should have discovered branch-a and branch-b even if they point to the same OID"
    );
}

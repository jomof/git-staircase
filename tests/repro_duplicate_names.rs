use git_staircase::core::persistence::write_metadata;
use git_staircase::git::GitRepo;
use git_staircase::model::{StaircaseMetadata, Step};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_duplicate_step_names_leak() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);

    // Create two commits
    fs::write(dir.join("a.txt"), "a").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "a"]);
    let c1 = run_git(dir, &["rev-parse", "HEAD"]);

    // Detach head so main doesn't keep c1 alive
    run_git(dir, &["checkout", "--detach"]);
    run_git(dir, &["branch", "-D", "main"]);

    fs::write(dir.join("b.txt"), "b").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "b"]);
    let c2 = run_git(dir, &["rev-parse", "HEAD"]);

    let repo = GitRepo::new(dir.to_path_buf());

    let metadata = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test-staircase".to_string(),
        target: "refs/heads/main".to_string(),
        steps: vec![
            Step {
                id: "s1".to_string(),
                name: "duplicate".to_string(),
                cut: c1.clone(),
                branch: None,
            },
            Step {
                id: "s2".to_string(),
                name: "duplicate".to_string(),
                cut: c2.clone(),
                branch: None,
            },
        ],
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        landing_policy: None,
    };

    write_metadata(&repo, &metadata).unwrap();

    // Check if c1 has any ref pointing to it.
    let refs = run_git(dir, &["for-each-ref", "--points-at", &c1]);
    assert!(
        !refs.is_empty(),
        "Commit c1 should have at least one ref pointing to it for reachability"
    );
}

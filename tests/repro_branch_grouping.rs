#![allow(unused_variables)]
use git_staircase::GitRepo;
use git_staircase::core::discovery::discover;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[ignore]
fn test_branch_grouping_at_same_commit() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Initialize repo
    Command::new("git")
        .current_dir(dir)
        .args(&["init", "-b", "main"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["config", "user.email", "test@example.com"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["config", "user.name", "Test"])
        .output()
        .unwrap();

    fs::write(dir.join("file.txt"), "base").unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["commit", "-m", "base"])
        .output()
        .unwrap();

    // Create two branches at the same commit descending from main
    fs::write(dir.join("file.txt"), "step1").unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["commit", "-m", "step1"])
        .output()
        .unwrap();
    let step1_oid = String::from_utf8_lossy(
        &Command::new("git")
            .current_dir(dir)
            .args(&["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();

    Command::new("git")
        .current_dir(dir)
        .args(&["branch", "b1", &step1_oid])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["branch", "b2", &step1_oid])
        .output()
        .unwrap();

    let repo = GitRepo::new(dir.to_path_buf());

    // ACT: discover staircases from main
    let discoveries = discover(&repo, Some("main"), None, false).unwrap();

    let found_combined = discoveries.iter().any(|d| {
        if let git_staircase::model::Discovery::Linear(s) = d {
            let names: std::collections::HashSet<_> =
                s.steps.iter().map(|step| step.name.as_str()).collect();
            names.contains("b1") && names.contains("b2")
        } else {
            false
        }
    });

    assert!(
        found_combined,
        "b1 and b2 at same commit were not grouped into one staircase!"
    );
}

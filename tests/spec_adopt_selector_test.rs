mod common;
use common::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_adopt_by_selector() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();

    // ARRANGE
    // Create a repository with a branch 'feature' ahead of 'main' to form an implicit staircase.
    run_git(repo_path, &["init", "-b", "main"]);
    fs::write(repo_path.join("init.txt"), "initial").unwrap();
    run_git(repo_path, &["add", "init.txt"]);
    run_git(repo_path, &["commit", "-m", "initial"]);

    run_git(repo_path, &["checkout", "-b", "feature"]);
    fs::write(repo_path.join("feature.txt"), "feature").unwrap();
    run_git(repo_path, &["add", "feature.txt"]);
    run_git(repo_path, &["commit", "-m", "feature"]);

    // ACT
    // Run 'git staircase adopt feature'.
    let (success, stdout, stderr) = run_staircase(repo_path, &["adopt", "feature"]);

    // ASSERT
    assert!(
        success,
        "adopt failed: stdout: {}, stderr: {}",
        stdout, stderr
    );

    // Verify that 'git staircase list' shows the staircase as managed.
    let (success, stdout, stderr) = run_staircase(repo_path, &["list", "--managed", "--porcelain"]);
    assert!(success, "list failed: {}", stderr);
    assert!(
        stdout.contains("feature\t"),
        "Staircase 'feature' should be in managed list: {}",
        stdout
    );
}

#[test]
fn test_adopt_with_rename() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();

    // ARRANGE
    run_git(repo_path, &["init", "-b", "main"]);
    fs::write(repo_path.join("init.txt"), "initial").unwrap();
    run_git(repo_path, &["add", "init.txt"]);
    run_git(repo_path, &["commit", "-m", "initial"]);

    run_git(repo_path, &["checkout", "-b", "feature"]);
    fs::write(repo_path.join("feature.txt"), "feature").unwrap();
    run_git(repo_path, &["add", "feature.txt"]);
    run_git(repo_path, &["commit", "-m", "feature"]);

    // ACT
    // Run 'git staircase adopt feature --name new-feature'.
    let (success, stdout, stderr) =
        run_staircase(repo_path, &["adopt", "feature", "--name", "new-feature"]);

    // ASSERT
    assert!(
        success,
        "adopt with rename failed: stdout: {}, stderr: {}",
        stdout, stderr
    );

    // Verify that 'git staircase list' shows the staircase as 'new-feature'.
    let (success, stdout, stderr) = run_staircase(repo_path, &["list", "--managed", "--porcelain"]);
    assert!(success, "list failed: {}", stderr);
    assert!(
        stdout.contains("new-feature\t"),
        "Staircase 'new-feature' should be in managed list: {}",
        stdout
    );

    // Old name 'feature' should not be present as a managed staircase name
    for line in stdout.lines() {
        assert!(
            !line.starts_with("feature\t"),
            "Old name 'feature' should not be the managed name in line: {}",
            line
        );
    }
}

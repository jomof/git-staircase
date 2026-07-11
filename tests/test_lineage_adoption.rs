mod common;
use common::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_lineage_id_adoption() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();

    // ARRANGE: Create a repository with two branches forming an implicit staircase
    run_git(repo_path, &["init", "-b", "main"]);
    fs::write(repo_path.join("init.txt"), "initial").unwrap();
    run_git(repo_path, &["add", "init.txt"]);
    run_git(repo_path, &["commit", "-m", "initial"]);
    
    run_git(repo_path, &["checkout", "-b", "feature/core"]);
    fs::write(repo_path.join("core.txt"), "core").unwrap();
    run_git(repo_path, &["add", "core.txt"]);
    run_git(repo_path, &["commit", "-m", "core"]);

    run_git(repo_path, &["checkout", "-b", "feature/ui"]);
    fs::write(repo_path.join("ui.txt"), "ui").unwrap();
    run_git(repo_path, &["add", "ui.txt"]);
    run_git(repo_path, &["commit", "-m", "ui"]);

    // Verify it is discovered as implicit and has an implicit ID
    let (success, status, stderr) = run_staircase(repo_path, &["status", "feature/ui", "--porcelain"]);
    assert!(success, "status failed: {}", stderr);
    // Porcelain output format: name \t id \t state
    let id_line = status.lines().next().unwrap();
    let parts: Vec<&str> = id_line.split('\t').collect();
    let implicit_id = parts[1];
    assert!(implicit_id.starts_with("implicit@"), "Expected implicit ID, got {}", implicit_id);

    // ACT: Run git staircase id --kind=lineage to trigger adoption
    let (success, lineage_id, stderr) = run_staircase(repo_path, &["id", "feature/ui", "--kind=lineage", "--porcelain"]);
    assert!(success, "id failed: {}", stderr);
    
    // ASSERT: Verify the returned ID is a UUID (not implicit@...)
    assert!(!lineage_id.starts_with("implicit@"), "Expected UUID, still got implicit ID: {}", lineage_id);
    // Basic UUID check: 8-4-4-4-12 hex chars
    assert!(uuid::Uuid::parse_str(&lineage_id).is_ok(), "Returned ID is not a valid UUID: {}", lineage_id);

    // ACT: Modify the staircase (rebase)
    run_git(repo_path, &["checkout", "main"]);
    fs::write(repo_path.join("main_new.txt"), "more main").unwrap();
    run_git(repo_path, &["add", "main_new.txt"]);
    run_git(repo_path, &["commit", "-m", "main updated"]);
    
    // Note: using the nominal name "feature" ensures we find the managed staircase
    let (success, _, stderr) = run_staircase(repo_path, &["rebase", "feature", "--to", "main"]);
    assert!(success, "rebase failed: {}", stderr);
    
    // ASSERT: Verify that id --kind=lineage returns the same UUID, even though content hash changed
    let (success, new_lineage_id, stderr) = run_staircase(repo_path, &["id", "feature", "--kind=lineage", "--porcelain"]);
    assert!(success, "id failed: {}", stderr);
    assert_eq!(lineage_id, new_lineage_id, "Lineage ID should be stable across rebase");
}

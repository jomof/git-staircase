use git_staircase::GitRepo;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[ignore]
fn test_stale_memoization() {
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

    fs::write(dir.join("file.txt"), "v1").unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["commit", "-m", "v1"])
        .output()
        .unwrap();
    let oid1 = Command::new("git")
        .current_dir(dir)
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let oid1 = String::from_utf8_lossy(&oid1.stdout).trim().to_string();

    let repo = GitRepo::new(dir.to_path_buf());

    // Resolve main - should be oid1
    let resolved1 = repo.resolve_commit("main").unwrap();
    assert_eq!(resolved1, oid1);

    // Update main externally
    fs::write(dir.join("file.txt"), "v2").unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["commit", "-m", "v2"])
        .output()
        .unwrap();
    let oid2 = Command::new("git")
        .current_dir(dir)
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let oid2 = String::from_utf8_lossy(&oid2.stdout).trim().to_string();

    assert_ne!(oid1, oid2);

    // Resolve main again
    let resolved2 = repo.resolve_commit("main").unwrap();

    // ASSERT: resolve_commit("main") returns the NEW OID
    assert_eq!(resolved2, oid2, "Memoized OID is stale!");
}

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn run_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed. Stderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
    let path = dir.join(file);
    fs::write(path, contents).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

fn setup_repo() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    tmp
}

fn run_staircase(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let bin = env!("CARGO_BIN_EXE_git-staircase");
    let output = Command::new(bin)
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_list_json() {
    let tmp = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let (success, stdout, stderr) = run_staircase(dir, &["list", "--json"]);
    assert!(success, "list --json failed: {}", stderr);

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json.is_array());
    let list = json.as_array().unwrap();
    assert!(!list.is_empty());
    // Find our implicit staircase
    let found = list.iter().any(|s| s["name"] == "feature/auth");
    assert!(found, "Should find feature/auth in the list: {}", stdout);
    assert_eq!(list[0]["management"], "implicit");
}

#[test]
fn test_status_json() {
    let tmp = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    // Discover and adopt
    let (success, _, stderr) = run_staircase(dir, &["adopt", "auth", "feature/auth-core"]);
    assert!(success, "adopt failed: {}", stderr);

    let (success, stdout, stderr) = run_staircase(dir, &["status", "auth", "--json"]);
    assert!(success, "status --json failed: {}", stderr);

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert_eq!(json["metadata"]["name"], "auth");
    assert!(json["is_clean"].as_bool().unwrap());
}

#[test]
fn test_status_porcelain() {
    let tmp = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_staircase(dir, &["adopt", "auth", "feature/auth-core"]);

    let (success, stdout, stderr) = run_staircase(dir, &["status", "auth", "--porcelain"]);
    assert!(success, "status --porcelain failed: {}", stderr);

    assert!(stdout.contains("auth"));
    assert!(stdout.contains("clean"));
    assert!(stdout.contains("step\tfeature/auth-core"));
}

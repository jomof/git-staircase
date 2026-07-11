mod common;
use common::*;

#[test]
fn test_list_json() {
    let (tmp, _repo) = setup_repo();
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
    let (tmp, _repo) = setup_repo();
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
    let (tmp, _repo) = setup_repo();
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

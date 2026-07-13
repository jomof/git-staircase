use crate::common::*;

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
    let found = list.iter().any(|s| s["metadata"]["name"] == "feature/auth");
    assert!(found, "Should find feature/auth in the list: {}", stdout);
    assert!(list[0]["is_implicit"].as_bool().unwrap());
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

#[test]
fn test_commits_json() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    let (success, stdout, stderr) = run_staircase(dir, &["commits", "feature/auth-core", "--json"]);
    assert!(success, "commits --json failed: {}", stderr);

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json["steps"].is_array());
    let steps = json["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["name"], "feature/auth-core");
    assert!(steps[0]["commits"].is_array());
}

#[test]
fn test_commits_porcelain() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    let (success, stdout, stderr) =
        run_staircase(dir, &["commits", "feature/auth-core", "--porcelain"]);
    assert!(success, "commits --porcelain failed: {}", stderr);

    assert!(stdout.contains("step\t1\tfeature/auth-core"));
    assert!(stdout.contains("commit\t"));
}

#[test]
fn test_log_json() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    let (success, stdout, stderr) = run_staircase(dir, &["log", "feature/auth-core", "--json"]);
    assert!(success, "log --json failed: {}", stderr);

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json.is_array());
    let list = json.as_array().unwrap();
    assert!(!list.is_empty());
}

#[test]
fn test_diff_json() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    let (success, stdout, stderr) = run_staircase(dir, &["diff", "feature/auth-core", "--json"]);
    assert!(success, "diff --json failed: {}", stderr);

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json.is_string());
    assert!(json.as_str().unwrap().contains("diff --git"));
}

#[test]
fn test_graph_json() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    let (success, stdout, stderr) = run_staircase(dir, &["graph", "feature/auth-core", "--json"]);
    assert!(success, "graph --json failed: {}", stderr);

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json.is_string());
    assert!(json.as_str().unwrap().contains("*"));
}

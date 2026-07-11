mod common;
use common::*;

#[test]
fn test_reorder_json() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let (success, stdout, stderr) = run_staircase(
        dir,
        &["--json", "reorder", "feature/auth", "--order", "2,1"],
    );

    assert!(success, "reorder --json failed: {}", stderr);
    println!("Stdout: '{}'", stdout);

    // It should output JSON representing the new state or at least a success status
    assert!(!stdout.trim().is_empty(), "JSON output should not be empty");
    serde_json::from_str::<serde_json::Value>(stdout.trim()).expect("Output should be valid JSON");
}

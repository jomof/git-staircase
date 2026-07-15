mod common;
use common::*;

#[test]
fn test_metadata_json_contains_symbolic_integration_target() {
    // ARRANGE: Create a staircase
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let name = "feature/auth";

    // ACT: Run show --json
    let (success, stdout, stderr) = run_staircase(dir, &["show", name, "--json"]);

    // ASSERT
    assert!(success, "show --json command failed: {}", stderr);

    // Parse JSON
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");

    let target = json.get("target");
    let symbolic_target = json.get("symbolic_integration_target");

    assert!(
        symbolic_target.is_some(),
        "JSON should contain 'symbolic_integration_target' field, but it was missing. Output: {}",
        stdout
    );
    assert!(
        target.is_none(),
        "JSON should NOT contain 'target' field, but it was present. Output: {}",
        stdout
    );
}

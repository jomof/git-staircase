use crate::common::*;

#[test]
fn test_consistent_json_output() {
    let ctx = TestContext::new();
    let root = ctx.commit("root.txt", "root", "root");
    ctx.run_git(&["checkout", "-b", "feature/1", &root]);
    let c1 = ctx.commit("1.txt", "1", "c1");
    ctx.run_git(&["checkout", "-b", "feature/2", &c1]);
    let _c2 = ctx.commit("2.txt", "2", "c2");

    // Test 'list --json'
    let (success, stdout, stderr) = ctx.run_staircase(&["list", "--json"]);
    assert!(success, "list --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_array(), "list --json should return an array");

    // Test 'status --json'
    let (success, stdout, stderr) = ctx.run_staircase(&["status", "feature/2", "--json"]);
    assert!(success, "status --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_object(), "status --json should return an object");

    // Test 'id --json'
    let (success, stdout, stderr) = ctx.run_staircase(&["id", "feature/2", "--json"]);
    assert!(success, "id --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_object(), "id --json should return an object");
    assert!(json.get("id").is_some());
}

#[test]
fn test_consistent_porcelain_output() {
    let ctx = TestContext::new();
    let root = ctx.commit("root.txt", "root", "root");
    ctx.run_git(&["checkout", "-b", "feature/1", &root]);
    ctx.run_git(&["branch", "--set-upstream-to=main"]);
    let _c1 = ctx.commit("1.txt", "1", "c1");

    // Test 'list --porcelain'
    let (success, stdout, _stderr) = ctx.run_staircase(&["list", "--porcelain"]);
    assert!(success);
    assert!(
        stdout.contains("feature/1\timplicit@"),
        "porcelain list should contain staircase info: {}",
        stdout
    );

    // Test 'status --porcelain'
    let (success, stdout, _stderr) = ctx.run_staircase(&["status", "feature/1", "--porcelain"]);
    assert!(success);
    assert!(
        stdout.contains("feature/1\timplicit@"),
        "porcelain status should contain staircase info: {}",
        stdout
    );

    // Test 'id --porcelain'
    let (success, stdout, _stderr) = ctx.run_staircase(&["id", "feature/1", "--porcelain"]);
    assert!(success);
    assert!(!stdout.trim().is_empty());
}

#[test]
fn test_adopt_output_consistency() {
    let ctx = TestContext::new();
    ctx.commit("root.txt", "root", "root");
    ctx.run_git(&["checkout", "-b", "feat1"]);
    ctx.run_git(&["branch", "--set-upstream-to=main"]);
    ctx.commit("f1.txt", "f1", "f1");

    // Test 'adopt' default human output
    let (success, stdout, stderr) = ctx.run_staircase(&["adopt", "my-sc", "feat1"]);
    assert!(success, "adopt failed: {}", stderr);
    assert!(
        stdout.contains("Name: my-sc"),
        "adopt output should contain Name: {}",
        stdout
    );

    // Test 'adopt --json'
    ctx.run_git(&["checkout", "main"]);
    ctx.run_git(&["checkout", "-b", "feat2"]);
    ctx.commit("f2.txt", "f2", "f2");
    let (success, stdout, stderr) =
        ctx.run_staircase(&["adopt", "my-sc2", "feat2", "--json"]);
    assert!(success, "adopt --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json.get("name").unwrap().as_str().unwrap(), "my-sc2");
}

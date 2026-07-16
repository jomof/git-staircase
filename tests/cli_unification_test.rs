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
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, file: &str, content: &str, msg: &str) -> String {
    fs::write(dir.join(file), content).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

fn run_staircase(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let ws_dir = std::env::temp_dir().join(format!(".ws_storage_{:p}", dir));
    let bin_str = env!("CARGO_BIN_EXE_git-staircase");
    let mut binary = std::path::PathBuf::from(bin_str);
    if bin_str.contains("/shadow-") || !binary.exists() {
        let fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join("git-staircase");
        if fallback.exists() {
            binary = fallback;
        }
    }
    let output = match Command::new(&binary)
        .current_dir(dir)
        .env("GIT_STAIRCASE_WORKSPACE_DIR", &ws_dir)
        .args(args)
        .output()
    {
        Ok(out) => out,
        Err(e) => panic!("Failed to run binary '{:?}' in dir '{:?}': {}", binary, dir, e),
    };
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_consistent_json_output() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    let root = commit(dir, "root.txt", "root", "root");
    run_git(dir, &["checkout", "-b", "feature/1", &root]);
    let c1 = commit(dir, "1.txt", "1", "c1");
    run_git(dir, &["checkout", "-b", "feature/2", &c1]);
    let _c2 = commit(dir, "2.txt", "2", "c2");

    // Test 'list --json'
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--json"]);
    assert!(success, "list --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_array(), "list --json should return an array");

    // Test 'status --json'
    let (success, stdout, stderr) = run_staircase(dir, &["status", "feature/2", "--json"]);
    assert!(success, "status --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_object(), "status --json should return an object");

    // Test 'id --json'
    let (success, stdout, stderr) = run_staircase(dir, &["id", "feature/2", "--json"]);
    assert!(success, "id --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_object(), "id --json should return an object");
    assert!(json.get("id").is_some());
}

#[test]
fn test_consistent_porcelain_output() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    let root = commit(dir, "root.txt", "root", "root");
    run_git(dir, &["checkout", "-b", "feature/1", &root]);
    let _c1 = commit(dir, "1.txt", "1", "c1");

    // Test 'list --porcelain'
    let (success, stdout, _stderr) = run_staircase(dir, &["list", "--porcelain"]);
    assert!(success);
    assert!(
        stdout.contains("feature/1\timplicit@"),
        "porcelain list should contain staircase info: {}",
        stdout
    );

    // Test 'status --porcelain'
    let (success, stdout, _stderr) = run_staircase(dir, &["status", "feature/1", "--porcelain"]);
    assert!(success);
    assert!(
        stdout.contains("feature/1\timplicit@"),
        "porcelain status should contain staircase info: {}",
        stdout
    );

    // Test 'id --porcelain'
    let (success, stdout, _stderr) = run_staircase(dir, &["id", "feature/1", "--porcelain"]);
    assert!(success);
    assert!(!stdout.trim().is_empty());
}

#[test]
fn test_adopt_output_consistency() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    commit(dir, "root.txt", "root", "root");
    run_git(dir, &["checkout", "-b", "feat1"]);
    commit(dir, "f1.txt", "f1", "f1");

    // Test 'adopt' default human output
    let (success, stdout, stderr) = run_staircase(dir, &["adopt", "my-sc", "feat1"]);
    assert!(success, "adopt failed: {}", stderr);
    assert!(
        stdout.contains("Name: my-sc"),
        "adopt output should contain Name: {}",
        stdout
    );

    // Test 'adopt --json'
    let (success, stdout, stderr) = run_staircase(dir, &["adopt", "my-sc2", "feat1", "--json"]);
    assert!(success, "adopt --json failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json.get("name").unwrap().as_str().unwrap(), "my-sc2");
}

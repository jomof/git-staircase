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
fn test_list_implicit_flag_and_output() {
    let tmp = setup_repo();
    let dir = tmp.path();

    // Create implicit staircase with 2 steps
    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // ACT: Run git staircase list --implicit
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--implicit"]);

    // ASSERT: Verify success and output format
    assert!(success, "list --implicit failed: {}", stderr);

    let output = stdout.trim();
    assert_eq!(output, "feature/auth 2 steps clean (implicit)");
}

#[test]
fn test_list_managed_output() {
    let tmp = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(dir, &["adopt", "auth", "feature/auth-core"]);
    assert!(success, "adopt failed: {}", stderr);

    // ACT: Run git staircase list --managed
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--managed"]);

    assert!(success, "list --managed failed: {}", stderr);

    let output = stdout.trim();
    // Expected: auth 1 step clean
    assert_eq!(output, "auth 1 step clean");
}

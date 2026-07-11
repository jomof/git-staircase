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
    run_git(path, &["init", "-b", "main"]);
    commit(path, "init.txt", "initial", "initial commit");
    tmp
}

fn run_staircase(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let binary = env!("CARGO_BIN_EXE_git-staircase");
    let output = Command::new(binary)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute git-staircase");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_inspection_commands_on_implicit_staircase() {
    // ARRANGE
    let tmp = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    run_git(dir, &["checkout", "-b", "feature/auth-tests"]);
    let c3 = commit(dir, "file3.txt", "3", "commit 3");

    let name = "feature/auth";

    // ACT & ASSERT: steps
    let (success, stdout, stderr) = run_staircase(dir, &["steps", name]);
    assert!(success, "steps command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains("feature/auth-tests"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains(&c2[..7]));
    assert!(stdout.contains(&c3[..7]));

    // ACT & ASSERT: commits
    let (success, stdout, stderr) = run_staircase(dir, &["commits", name]);
    assert!(success, "commits command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains(&c2[..7]));
    assert!(stdout.contains("feature/auth-tests"));
    assert!(stdout.contains(&c3[..7]));

    // ACT & ASSERT: log
    let (success, stdout, stderr) = run_staircase(dir, &["log", name]);
    assert!(success, "log command failed: {}", stderr);
    assert!(stdout.contains("commit 1"));
    assert!(stdout.contains("commit 2"));
    assert!(stdout.contains("commit 3"));
    assert!(!stdout.contains("initial commit")); // Should only show commits in the staircase

    // ACT & ASSERT: diff
    let (success, stdout, stderr) = run_staircase(dir, &["diff", name]);
    assert!(success, "diff command failed: {}", stderr);
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("file3.txt"));

    // ACT & ASSERT: graph
    let (success, stdout, stderr) = run_staircase(dir, &["graph", name]);
    assert!(success, "graph command failed: {}", stderr);
    assert!(stdout.contains("commit 1"));
    assert!(stdout.contains("commit 2"));
    assert!(stdout.contains("commit 3"));
}

#[test]
fn test_inspection_commands_on_managed_staircase() {
    // ARRANGE
    let tmp = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(
        dir,
        &[
            "adopt",
            "auth",
            "--onto",
            "main",
            "feature/auth-core",
            "feature/auth-ui",
        ],
    );
    assert!(success, "adopt failed: {}", stderr);

    let name = "auth";

    // ACT & ASSERT: steps
    let (success, stdout, stderr) = run_staircase(dir, &["steps", name]);
    assert!(success, "steps command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains(&c2[..7]));

    // ACT & ASSERT: commits
    let (success, stdout, stderr) = run_staircase(dir, &["commits", name]);
    assert!(success, "commits command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains(&c2[..7]));
}

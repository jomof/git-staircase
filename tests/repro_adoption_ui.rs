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

fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
    let path = dir.join(file);
    fs::write(path, contents).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

#[test]
fn test_adoption_ui() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    commit(dir, "init.txt", "initial", "initial commit");

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // We need to run the compiled binary.
    // Assuming 'cargo build' was run and the binary is in target/debug/git-staircase
    // But I can't easily find the binary path here.
    // I'll just use 'cargo run' instead.

    let output = Command::new("cargo")
        .current_dir(dir)
        .args(["run", "--", "id", "feature/auth", "--kind", "lineage"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Stdout: {}", stdout);

    // The spec says it should print "adopted implicit staircase 'feature/auth'"
    assert!(stdout.contains("adopted implicit staircase 'feature/auth'"));
}

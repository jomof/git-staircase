use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_reorder_underflow_error() {
    let dir = TempDir::new().unwrap();
    let repo_path = dir.path().to_path_buf();

    let run = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(&repo_path)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    };

    run(&["init", "-b", "main"]);
    run(&["config", "user.email", "you@example.com"]);
    run(&["config", "user.name", "Your Name"]);
    run(&["commit", "--allow-empty", "-m", "initial commit"]);
    run(&["checkout", "-b", "branch-a"]);
    fs::write(repo_path.join("file"), "content").unwrap();
    run(&["add", "file"]);
    run(&["commit", "-m", "work"]);

    let bin = env!("CARGO_BIN_EXE_git-staircase");
    let output = Command::new(bin)
        .arg("reorder")
        .arg("branch-a")
        .arg("--steps")
        .arg("0,1")
        .arg("--onto")
        .arg("main")
        .env("GIT_DIR", repo_path.join(".git"))
        .env("GIT_WORK_TREE", &repo_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Step indices must be 1-based"),
        "Should have returned clean error but got: {}",
        stderr
    );
}

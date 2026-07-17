use git_staircase::git::GitRepo;
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[test]
fn test_verify_no_timeout() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path().join("repo");
    fs::create_dir(&repo_path).unwrap();
    let repo = GitRepo::new(repo_path.clone());
    repo.run(&["init"]).unwrap();

    let file_path = repo_path.join("file.txt");
    fs::write(&file_path, "test").unwrap();
    repo.run(&["add", "."]).unwrap();
    repo.run(&["commit", "-m", "init"]).unwrap();

    // Create a draft
    fs::write(&file_path, "draft").unwrap();
    repo.run(&["add", "."]).unwrap();

    // Run the CLI
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_git-staircase"));
    cmd.current_dir(&repo_path);
    cmd.args(&[
        "verify",
        "--draft",
        "--timeout",
        "2",
        "--build-command",
        "sleep 10",
    ]);

    let start = Instant::now();
    let mut child = cmd.spawn().unwrap();

    // We expect the command to time out in, say, 2 seconds if timeouts were implemented.
    // If it takes more than 5 seconds, it's hanging.
    let _res = child.wait().unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "verify command hung for {:?} because of missing timeout mechanism",
        elapsed
    );
}

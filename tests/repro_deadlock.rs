use git_staircase::git::GitRepo;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_run_with_stdin_deadlock() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().to_path_buf();

    Command::new("git")
        .current_dir(&repo_path)
        .args(["init", "-b", "main"])
        .status()
        .unwrap();

    let repo = GitRepo::new(repo_path);

    // Create a large amount of input for a command that echoes back
    // We'll use 'cat-file --batch-check' which outputs a line for each input line
    let mut large_input = String::new();
    for _ in 0..100000 {
        large_input.push_str("HEAD\n");
    }

    // This should deadlock and timeout if the pipe buffer fills up
    let result = repo.run_with_stdin(&["cat-file", "--batch-check"], &large_input);
    assert!(result.is_ok());
}

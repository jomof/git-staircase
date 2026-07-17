use git_staircase::GitRepo;

#[test]
fn test_git_command_error_quoting() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(&["init"])
        .output()
        .unwrap();

    let repo = GitRepo::new(dir.to_path_buf());

    // Run a command that will fail
    let res = repo
        .command()
        .args(&["rev-parse", "--verify", "invalid ref with spaces"])
        .run();

    match res {
        Err(e) => {
            let msg = e.to_string();
            println!("Error message: {}", msg);
            // The command should ideally be quoted: git rev-parse --verify "invalid ref with spaces"
            // But currently it is: git rev-parse --verify invalid ref with spaces
            assert!(
                msg.contains("git rev-parse --verify \"invalid ref with spaces\"")
                    || msg.contains("git rev-parse --verify 'invalid ref with spaces'"),
                "Error message should contain quoted arguments: {}",
                msg
            );
        }
        Ok(_) => panic!("Command should have failed"),
    }
}

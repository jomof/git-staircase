use git_staircase::GitRepo;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cat_file_trimming() {
    // ARRANGE: Initialize a git repo and write a blob with trailing newlines
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .unwrap();

    let content = "line1\nline2\n\n";
    let oid = {
        let mut child = Command::new("git")
            .args(&["hash-object", "-w", "--stdin"])
            .current_dir(repo_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(content.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };

    let repo = GitRepo::new(repo_path.to_path_buf());

    // ACT: Read the blob back using repo.run() which currently trims by default
    let read_content = repo.run(&["cat-file", "-p", &oid]).unwrap();

    // ASSERT: Verify that the content was NOT trimmed (this currently FAILS)
    assert_eq!(read_content, content, "Content was trimmed unexpectedly!");
}

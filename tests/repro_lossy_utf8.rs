use git_staircase::GitRepo;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cat_file_binary_data() {
    // ARRANGE
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().to_path_buf();

    Command::new("git")
        .current_dir(&repo_path)
        .args(&["init", "-b", "main"])
        .output()
        .unwrap();

    let repo = GitRepo::new(repo_path.clone());

    // Create a binary blob with invalid UTF-8 (e.g. 0xFF)
    let binary_data = vec![0u8, 1, 2, 0xFF, 0xFE, 4, 5];

    let mut child = Command::new("git")
        .current_dir(&repo_path)
        .args(&["hash-object", "-w", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&binary_data).unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let oid = String::from_utf8(output.stdout).unwrap().trim().to_string();

    // ACT
    let read_data_res = repo.cat_file_bytes(&oid);

    // ASSERT
    assert!(read_data_res.is_ok());
    let read_data = read_data_res.unwrap();

    assert_eq!(
        read_data, binary_data,
        "Read data should match original binary data exactly"
    );
}

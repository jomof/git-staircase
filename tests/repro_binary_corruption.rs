use git_staircase::git::GitRepo;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn setup_repo() -> (TempDir, GitRepo) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    Command::new("git")
        .current_dir(&path)
        .args(&["init", "-b", "main"])
        .output()
        .unwrap();
    let repo = GitRepo::new(path);
    (tmp, repo)
}

#[test]
fn test_binary_blob_corruption() {
    let (tmp, repo) = setup_repo();
    let dir = tmp.path();

    // Create a binary file with invalid UTF-8 sequences
    let binary_data = vec![0u8, 155u8, 255u8, 0u8];
    let file_path = dir.join("binary.bin");
    fs::write(&file_path, &binary_data).unwrap();

    Command::new("git")
        .current_dir(dir)
        .args(&["add", "binary.bin"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(dir)
        .args(&["commit", "-m", "binary"])
        .output()
        .unwrap();

    let oid_output = Command::new("git")
        .current_dir(dir)
        .args(&["rev-parse", "HEAD:binary.bin"])
        .output()
        .unwrap();
    let oid = String::from_utf8(oid_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // Read it back with git-staircase
    let read_back = repo.cat_file(&oid).unwrap();

    // String::from_utf8_lossy will have replaced invalid bytes
    assert_eq!(
        read_back, binary_data,
        "Binary data corrupted! Expected {:?}, got {:?}",
        binary_data, read_back
    );
}

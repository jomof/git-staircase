use crate::common::run_git;
use git_staircase::git::GitRepo;
use std::fs;
use tempfile::TempDir;

fn setup_repo() -> (TempDir, GitRepo) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path();
    run_git(path, &["init", "-b", "main"]);
    let repo = GitRepo::new(path.to_path_buf());
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

    run_git(dir, &["add", "binary.bin"]);
    run_git(dir, &["commit", "-m", "binary"]);

    let oid = run_git(dir, &["rev-parse", "HEAD:binary.bin"]);

    // Read it back with git-staircase
    let read_back = repo.cat_file(&oid).unwrap();

    // String::from_utf8_lossy will have replaced invalid bytes
    assert_eq!(
        read_back, binary_data,
        "Binary data corrupted! Expected {:?}, got {:?}",
        binary_data, read_back
    );
}

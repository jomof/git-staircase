use git_staircase::GitRepo;
use git_staircase::git::TreeEntry;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_write_tree_special_chars() {
    let dir = tempdir().unwrap();
    let workdir = dir.path().to_path_buf();

    Command::new("git")
        .arg("init")
        .current_dir(&workdir)
        .output()
        .unwrap();

    let repo = GitRepo::new(workdir.clone());
    let oid = repo.write_blob("content").unwrap();

    let entries = vec![TreeEntry::blob(oid, "a\tb")];

    let result = repo.write_tree(&entries);
    assert!(
        result.is_ok(),
        "write_tree failed for entry with tab: {:?}",
        result.err()
    );
    let tree_oid = result.unwrap();

    let ls_tree = String::from_utf8(
        Command::new("git")
            .args(&["ls-tree", &tree_oid])
            .current_dir(&workdir)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert!(
        ls_tree.contains("a\tb"),
        "ls-tree output does not contain the expected name (might be incorrectly quoted or escaped): {:?}",
        ls_tree
    );
}

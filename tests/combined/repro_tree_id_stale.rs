use git_staircase::git::GitRepo;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_repro_tree_id_stale_bug() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    // Initialize repo
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("test")
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create commit A
    std::fs::write(repo_path.join("file.txt"), "A").unwrap();
    Command::new("git")
        .arg("add")
        .arg("file.txt")
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("A")
        .current_dir(repo_path)
        .output()
        .unwrap();
    let tree_a = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD^{tree}")
        .current_dir(repo_path)
        .output()
        .unwrap()
        .stdout;
    let tree_a = String::from_utf8(tree_a).unwrap().trim().to_string();

    let repo = GitRepo::new(repo_path.to_path_buf());

    // Get tree ID for HEAD
    let res_a = repo.get_tree_id("HEAD").unwrap();
    assert_eq!(res_a, tree_a);

    // Create commit B
    std::fs::write(repo_path.join("file.txt"), "B").unwrap();
    Command::new("git")
        .arg("add")
        .arg("file.txt")
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("B")
        .current_dir(repo_path)
        .output()
        .unwrap();
    let tree_b = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD^{tree}")
        .current_dir(repo_path)
        .output()
        .unwrap()
        .stdout;
    let tree_b = String::from_utf8(tree_b).unwrap().trim().to_string();

    assert_ne!(tree_a, tree_b);

    // Get tree ID for HEAD again. Stale cache returns tree of A.
    let res_b = repo.get_tree_id("HEAD").unwrap();

    assert_eq!(
        res_b, tree_b,
        "Should have gotten tree of B, but got tree of A (stale cache)!"
    );
}

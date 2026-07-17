use git_staircase::git::GitRepo;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_repro_ancestry_exclusion_bug() {
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

    // Create A -> B -> C
    Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg("A")
        .current_dir(repo_path)
        .output()
        .unwrap();
    let a_oid = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()
        .unwrap()
        .stdout;
    let a_oid = String::from_utf8(a_oid).unwrap().trim().to_string();

    Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg("B")
        .current_dir(repo_path)
        .output()
        .unwrap();
    let b_oid = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()
        .unwrap()
        .stdout;
    let b_oid = String::from_utf8(b_oid).unwrap().trim().to_string();

    Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg("C")
        .current_dir(repo_path)
        .output()
        .unwrap();
    let c_oid = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()
        .unwrap()
        .stdout;
    let c_oid = String::from_utf8(c_oid).unwrap().trim().to_string();

    let repo = GitRepo::new(repo_path.to_path_buf());

    // Call preload_ancestry_ext(oids=[C], exclude_oids=[B, A])
    // This incorrectly causes (A, C) to be memoized as false because A is excluded from rev-list.
    repo.preload_ancestry_ext(&[&c_oid], &[&b_oid, &a_oid])
        .unwrap();

    // Check if A is ancestor of C
    let is_anc = repo.is_ancestor(&a_oid, &c_oid).unwrap();

    assert!(is_anc, "A should be an ancestor of C!");
}

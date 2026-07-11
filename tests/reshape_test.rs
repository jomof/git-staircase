use git_staircase::GitRepo;
use git_staircase::core;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn run_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed. Stderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
    let path = dir.join(file);
    fs::write(path, contents).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

fn setup_repo() -> (TempDir, GitRepo) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    (tmp, GitRepo::new(path))
}

#[test]
fn test_reorder() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create three steps: main -> step1 -> step2 -> step3
    run_git(dir, &["checkout", "-b", "step1"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    let _c2 = commit(dir, "file2.txt", "2", "commit 2");
    run_git(dir, &["checkout", "-b", "step3"]);
    let _c3 = commit(dir, "file3.txt", "3", "commit 3");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");

    // Reorder: 1, 3, 2
    core::reorder(&repo, &rs, &[0, 2, 1]).expect("Reorder failed");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");
    let status = core::get_status_metadata(&repo, rs.metadata().clone()).unwrap();
    assert_eq!(status.metadata.steps.len(), 3);

    // Expected order: step1, step3, step2
    assert_eq!(status.metadata.steps[0].name, "step1");
    assert_eq!(status.metadata.steps[1].name, "step3");
    assert_eq!(status.metadata.steps[2].name, "step2");

    let new_c1 = &status.metadata.steps[0].cut;
    let new_c3 = &status.metadata.steps[1].cut;
    let new_c2 = &status.metadata.steps[2].cut;

    // c1 should remain the same as it's the first in reorder and it was first originally
    assert_eq!(new_c1, &c1);

    // Check ancestry: main -> new_c1 -> new_c3 -> new_c2
    let main_oid = repo.resolve_ref("main").unwrap();
    assert!(repo.is_ancestor(&main_oid, new_c1).unwrap());
    assert!(repo.is_ancestor(new_c1, new_c3).unwrap());
    assert!(repo.is_ancestor(new_c3, new_c2).unwrap());

    // Verify branch tips are updated
    assert_eq!(repo.resolve_ref("refs/heads/step1").unwrap(), *new_c1);
    assert_eq!(repo.resolve_ref("refs/heads/step3").unwrap(), *new_c3);
    assert_eq!(repo.resolve_ref("refs/heads/step2").unwrap(), *new_c2);

    // Verify file contents at the top
    run_git(dir, &["checkout", "step2"]);
    assert_eq!(fs::read_to_string(dir.join("file1.txt")).unwrap(), "1");
    assert_eq!(fs::read_to_string(dir.join("file2.txt")).unwrap(), "2");
    assert_eq!(fs::read_to_string(dir.join("file3.txt")).unwrap(), "3");
}

#[test]
fn test_drop() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create three steps: main -> step1 -> step2 -> step3
    run_git(dir, &["checkout", "-b", "step1"]);
    commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    commit(dir, "file2.txt", "2", "commit 2");
    run_git(dir, &["checkout", "-b", "step3"]);
    commit(dir, "file3.txt", "3", "commit 3");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");

    // Drop step 2
    core::drop(&repo, &rs, 1).expect("Drop failed");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");
    let status = core::get_status_metadata(&repo, rs.metadata().clone()).unwrap();
    assert_eq!(status.metadata.steps.len(), 2);
    assert_eq!(status.metadata.steps[0].name, "step1");
    assert_eq!(status.metadata.steps[1].name, "step3");

    let new_c1 = &status.metadata.steps[0].cut;
    let new_c3 = &status.metadata.steps[1].cut;

    // Check ancestry: main -> new_c1 -> new_c3
    let main_oid = repo.resolve_ref("main").unwrap();
    assert!(repo.is_ancestor(&main_oid, new_c1).unwrap());
    assert!(repo.is_ancestor(new_c1, new_c3).unwrap());

    // Verify file contents
    run_git(dir, &["checkout", "step3"]);
    assert_eq!(fs::read_to_string(dir.join("file1.txt")).unwrap(), "1");
    assert!(dir.join("file2.txt").exists() == false);
    assert_eq!(fs::read_to_string(dir.join("file3.txt")).unwrap(), "3");
}

#[test]
fn test_move() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create two steps: main -> step1 -> step2
    run_git(dir, &["checkout", "-b", "step1"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    let c2_1 = commit(dir, "file2_1.txt", "2.1", "commit 2.1");
    let _c2_2 = commit(dir, "file2_2.txt", "2.2", "commit 2.2");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");

    // Move c2_1 from step 2 to step 1
    core::move_commits(&repo, &rs, 1, 0, &[c2_1.clone()]).expect("Move failed");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");
    let status = core::get_status_metadata(&repo, rs.metadata().clone()).unwrap();
    assert_eq!(status.metadata.steps.len(), 2);
    assert_eq!(status.metadata.steps[0].name, "step1");
    assert_eq!(status.metadata.steps[1].name, "step2");

    let new_c1 = &status.metadata.steps[0].cut;
    let new_c2 = &status.metadata.steps[1].cut;

    assert_eq!(new_c1, &c2_1);

    // Check ancestry: main -> c1 -> new_c1 (c2_1) -> new_c2
    let main_oid = repo.resolve_ref("main").unwrap();
    assert!(repo.is_ancestor(&main_oid, &c1).unwrap());
    assert!(repo.is_ancestor(&c1, new_c1).unwrap());
    assert!(repo.is_ancestor(new_c1, new_c2).unwrap());

    // Verify file contents
    run_git(dir, &["checkout", "step1"]);
    assert!(dir.join("file2_1.txt").exists());
    assert!(dir.join("file2_2.txt").exists() == false);
}

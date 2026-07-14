use git_staircase::GitRepo;
use git_staircase::core::restack::{RestackOptions, RestackStrategy, Restacker};
use git_staircase::model::Step;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_manual_restack_corruption() {
    let dir = tempdir().unwrap();
    let repo_dir = dir.path();

    std::process::Command::new("git")
        .arg("init")
        .current_dir(repo_dir)
        .output()
        .unwrap();

    let repo = GitRepo::new(repo_dir.to_path_buf());

    // 1. Create a base commit
    fs::write(repo_dir.join("base"), "base content\n").unwrap();
    repo.run(&["add", "base"]).unwrap();
    repo.run(&["commit", "-m", "base"]).unwrap();
    let base_oid = repo.resolve_commit("HEAD").unwrap();

    // 2. Create two commits in a row
    fs::write(repo_dir.join("file1"), "content 1\n").unwrap();
    repo.run(&["add", "file1"]).unwrap();
    repo.run(&["commit", "-m", "c1"]).unwrap();
    let _c1 = repo.resolve_commit("HEAD").unwrap();

    fs::write(repo_dir.join("file2"), "content 2\n").unwrap();
    repo.run(&["add", "file2"]).unwrap();
    repo.run(&["commit", "-m", "c2"]).unwrap();
    let c2 = repo.resolve_commit("HEAD").unwrap();

    // 3. Create a target commit to restack onto
    repo.run(&["checkout", &base_oid]).unwrap();
    fs::write(repo_dir.join("target"), "target content\n").unwrap();
    repo.run(&["add", "target"]).unwrap();
    repo.run(&["commit", "-m", "target"]).unwrap();
    let target_oid = repo.resolve_commit("HEAD").unwrap();

    // 4. Try to restack c1 and c2 onto target using Manual strategy
    let restacker = Restacker::prepare(&repo, &[]).unwrap();
    let mut steps = vec![Step {
        id: "step".to_string(),
        name: "step".to_string(),
        cut: c2.clone(),
        branch: None,
    }];

    // In this case, perform_restack will call restack_step for the whole range base_oid..c2
    // It will use base_oid as old_parent_oid.
    let result = restacker.perform_restack(
        "test",
        &mut steps,
        &target_oid,
        &vec![base_oid.clone()],
        &RestackOptions {
            strategy: RestackStrategy::Manual,
            leave_upper_steps_stale: false,
        },
    );

    assert!(result.is_ok(), "Restack failed: {:?}", result.err());

    // 5. Verify the content of the restacked commits
    let new_c2 = steps[0].cut.clone();

    // Check files at new_c2
    let files = repo
        .run(&["ls-tree", "-r", "--name-only", &new_c2])
        .unwrap();
    let files: Vec<&str> = files.lines().collect();

    assert!(files.contains(&"base"));
    assert!(files.contains(&"target"));
    assert!(
        files.contains(&"file1"),
        "file1 missing in restacked commit"
    );
    assert!(
        files.contains(&"file2"),
        "file2 missing in restacked commit"
    );

    // The bug will cause file1 to be considered a conflict or duplicate if we're lucky,
    // but actually with --merge-base P X C2 where P is base, X is c1' and C2 is c2.
    // merge-tree will try to merge P->c1' (which is just file1) and P->c2 (which is file1 and file2).
    // This MIGHT actually work without conflicts because it's an "add-add" of the same file.
    // BUT if file1 was MODIFIED in both C1 and C2, it would definitely fail.
}

#[test]
fn test_manual_restack_corruption_modification() {
    let dir = tempdir().unwrap();
    let repo_dir = dir.path();

    std::process::Command::new("git")
        .arg("init")
        .current_dir(repo_dir)
        .output()
        .unwrap();

    let repo = GitRepo::new(repo_dir.to_path_buf());

    // 1. Create a base commit
    fs::write(repo_dir.join("file"), "line 1\n").unwrap();
    repo.run(&["add", "file"]).unwrap();
    repo.run(&["commit", "-m", "base"]).unwrap();
    let base_oid = repo.resolve_commit("HEAD").unwrap();

    // 2. Create two commits modifying the same file
    fs::write(repo_dir.join("file"), "line 1\nline 2\n").unwrap();
    repo.run(&["add", "file"]).unwrap();
    repo.run(&["commit", "-m", "c1"]).unwrap();
    let _c1 = repo.resolve_commit("HEAD").unwrap();

    fs::write(repo_dir.join("file"), "line 1\nline 2\nline 3\n").unwrap();
    repo.run(&["add", "file"]).unwrap();
    repo.run(&["commit", "-m", "c2"]).unwrap();
    let c2 = repo.resolve_commit("HEAD").unwrap();

    // 3. Create a target commit to restack onto
    repo.run(&["checkout", &base_oid]).unwrap();
    fs::write(repo_dir.join("other"), "other\n").unwrap();
    repo.run(&["add", "other"]).unwrap();
    repo.run(&["commit", "-m", "target"]).unwrap();
    let target_oid = repo.resolve_commit("HEAD").unwrap();

    // 4. Try to restack c1 and c2 onto target using Manual strategy
    let restacker = Restacker::prepare(&repo, &[]).unwrap();
    let mut steps = vec![Step {
        id: "step".to_string(),
        name: "step".to_string(),
        cut: c2.clone(),
        branch: None,
    }];

    let result = restacker.perform_restack(
        "test",
        &mut steps,
        &target_oid,
        &vec![base_oid.clone()],
        &RestackOptions {
            strategy: RestackStrategy::Manual,
            leave_upper_steps_stale: false,
        },
    );

    // This should fail because of the bug
    assert!(
        result.is_err(),
        "Restack should have failed due to incorrect merge base"
    );
    // If it didn't fail, it might have created a commit with conflict markers!
    if let Ok(_) = result {
        let new_c2 = &steps[0].cut;
        let content = repo
            .run(&["cat-file", "-p", &format!("{}:file", new_c2)])
            .unwrap();
        assert!(
            !content.contains("<<<<<<<"),
            "Conflict markers found in restacked file:\n{}",
            content
        );
    }
}

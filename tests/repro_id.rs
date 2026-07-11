use git_staircase::core::resolve_staircase;
use git_staircase::git::GitRepo;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_implicit_sub_staircase_id_mismatch() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();
    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .status()
            .unwrap();
        assert!(status.success());
    };
    run_git(&["init", "-b", "main"]);
    fs::write(repo_path.join("file"), "0").unwrap();
    run_git(&["add", "file"]);
    run_git(&["commit", "-m", "initial"]);
    run_git(&["checkout", "-b", "feature"]);
    fs::write(repo_path.join("file"), "1").unwrap();
    run_git(&["commit", "-am", "step 1"]);
    run_git(&["branch", "step-1"]);
    fs::write(repo_path.join("file"), "2").unwrap();
    run_git(&["commit", "-am", "step 2"]);
    run_git(&["branch", "step-2"]);

    let repo = GitRepo::new(repo_path.to_path_buf());
    let step1_oid = repo.resolve_commit("step-1").unwrap();
    let rs1 = resolve_staircase(&repo, &step1_oid, Some("main"))
        .unwrap()
        .unwrap();
    let id1_metadata = rs1.metadata().id.clone();
    println!("ID 1: {}", id1_metadata);

    let step2_oid = repo.resolve_commit("step-2").unwrap();
    let rs2 = resolve_staircase(&repo, &step2_oid, Some("main"))
        .unwrap()
        .unwrap();
    let id2_metadata = rs2.metadata().id.clone();
    println!("ID 2: {}", id2_metadata);

    // The sub-staircase (rs1) should have a different structural ID than the full staircase (rs2)
    assert_ne!(
        id1_metadata, id2_metadata,
        "Metadata ID should be unique to the steps content and updated on truncation"
    );
}

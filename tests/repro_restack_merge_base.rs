use git_staircase::core::restack::{RestackStrategy, Restacker};
use git_staircase::git::GitRepo;
use git_staircase::model::Step;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_restack_multiple_commits_in_step() {
    let dir = tempdir().expect("failed to create temp dir");
    let repo_path = dir.path();

    // Initialize git repo
    let run_git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .status()
            .expect("git command failed");
        assert!(status.success());
    };

    run_git(&["init"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test User"]);

    // Base commit
    fs::write(repo_path.join("file.txt"), "base\n").unwrap();
    run_git(&["add", "file.txt"]);
    run_git(&["commit", "-m", "base"]);
    let base_oid = GitRepo::new(repo_path.to_path_buf())
        .resolve_commit("HEAD")
        .unwrap();

    // Step with 2 commits
    fs::write(repo_path.join("file.txt"), "base\nline 1\n").unwrap();
    run_git(&["add", "file.txt"]);
    run_git(&["commit", "-m", "commit 1"]);

    fs::write(repo_path.join("file.txt"), "base\nline 1 modified\n").unwrap();
    run_git(&["add", "file.txt"]);
    run_git(&["commit", "-m", "commit 2"]);
    let step_cut = GitRepo::new(repo_path.to_path_buf())
        .resolve_commit("HEAD")
        .unwrap();

    // New base to restack onto
    run_git(&["checkout", "-b", "new-base", &base_oid]);
    fs::write(repo_path.join("other.txt"), "something else\n").unwrap();
    run_git(&["add", "other.txt"]);
    run_git(&["commit", "-m", "new base commit"]);
    let new_base_oid = GitRepo::new(repo_path.to_path_buf())
        .resolve_commit("HEAD")
        .unwrap();

    let repo = GitRepo::new(repo_path.to_path_buf());
    let restacker = Restacker::prepare(&repo, &[]).expect("failed to prepare restacker");

    let step = Step {
        id: "step-1".to_string(),
        name: "step-1".to_string(),
        cut: step_cut.clone(),
        branch: None,
    };

    // This should succeed, but it will fail if it uses the wrong merge base
    let result = restacker.restack_step(
        &step,
        &step_cut,
        &base_oid,
        &new_base_oid,
        RestackStrategy::Manual,
    );

    assert!(result.is_ok(), "Restack failed: {:?}", result.err());
    let new_step_oid = result.unwrap();

    // Check if the resulting commit has conflict markers in file.txt
    let content = repo
        .run(&["cat-file", "-p", &format!("{}:file.txt", new_step_oid)])
        .unwrap();
    assert!(
        !content.contains("<<<<<<<"),
        "Resulting commit contains conflict markers!\nContent:\n{}",
        content
    );
}

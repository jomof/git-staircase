use git_staircase::GitRepo;
use git_staircase::core::{self, ResolvedStaircase};
use git_staircase::model::{StaircaseMetadata, Step};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_reorder_data_loss_on_dirty_workdir() {
    // ARRANGE
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);

    fs::write(dir.join("file.txt"), "base").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "initial"]);

    fs::write(dir.join("file.txt"), "step1").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "step1"]);
    let s1_oid = run_git(dir, &["rev-parse", "HEAD"]);

    // Dirty change to a TRACKED file
    fs::write(dir.join("file.txt"), "dirty precious data").unwrap();

    let repo = GitRepo::new(dir.to_path_buf());
    let metadata = StaircaseMetadata {
        id: "test".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            name: "s1".to_string(),
            cut: s1_oid.clone(),
            branch: None,
        }],
        verification_policy: None,
    };
    let rs = ResolvedStaircase::Implicit(metadata);

    // ACT
    // Run reorder. It will trigger a rebase failure or just the finalize() call.
    let _ = core::manipulation::reorder(&repo, &rs, &[0]);

    // ASSERT
    let content = fs::read_to_string(dir.join("file.txt")).unwrap();
    assert_eq!(
        content, "dirty precious data",
        "Data loss! file.txt was reverted by checkout -f"
    );
}

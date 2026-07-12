use git_staircase::ToHuman;
use git_staircase::core::{persistence, status};
use git_staircase::git::GitRepo;
use git_staircase::model::{IdentityKind, StaircaseMetadata, Step, VerificationResult};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_verification_status_in_show_output() {
    // ARRANGE: Create a managed staircase and record verification
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    Command::new("git")
        .current_dir(repo_path)
        .args(["init", "-b", "main"])
        .status()
        .unwrap();

    let repo = GitRepo::new(repo_path.to_path_buf());

    // Create dummy commit
    fs::write(repo_path.join("file.txt"), "content").unwrap();
    repo.run(&["add", "file.txt"]).unwrap();
    repo.run(&["commit", "-m", "initial", "--no-gpg-sign"])
        .unwrap();
    let initial_oid = repo.resolve_commit("HEAD").unwrap();

    let metadata = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test-staircase".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "step1".to_string(),
            cut: initial_oid.clone(),
            branch: None,
        }],
        verification_policy: None,
    };
    persistence::write_metadata(&repo, &metadata).unwrap();

    let results = vec![VerificationResult {
        step_name: "step1".to_string(),
        cut: initial_oid.clone(),
        success: true,
        stdout: "Build success".to_string(),
        stderr: "".to_string(),
    }];
    // USE THE ID AS KEY, consistent with verify.rs
    persistence::record_verification(&repo, "test-id", IdentityKind::Lineage, &results).unwrap();

    // ACT: Get status and convert to human-readable string
    let status = status::get_status(&repo, "test-staircase").unwrap();
    let human_output = status.to_human();

    // ASSERT: Verify that the output includes verification info
    assert!(
        human_output.contains("verification"),
        "Output should contain 'verification' section. Actual output:\n{}",
        human_output
    );
    assert!(
        human_output.contains("step1: PASS"),
        "Output should show step1 passed. Actual output:\n{}",
        human_output
    );
}

use git_staircase::GitRepo;
use git_staircase::core;
use git_staircase::core::ResolvedStaircase;
use git_staircase::model::{StaircaseMetadata, Step};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn setup_repo() -> (TempDir, GitRepo) {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().to_path_buf();

    Command::new("git")
        .current_dir(&repo_path)
        .args(["init", "-b", "main"])
        .status()
        .unwrap();

    Command::new("git")
        .current_dir(&repo_path)
        .args(["config", "user.email", "test@example.com"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["config", "user.name", "Test"])
        .status()
        .unwrap();

    // Initial commit
    fs::write(repo_path.join("f"), "root\n").unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["add", "f"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "-m", "initial"])
        .status()
        .unwrap();

    // Step A
    Command::new("git")
        .current_dir(&repo_path)
        .args(["checkout", "-b", "branch-a"])
        .status()
        .unwrap();
    fs::write(repo_path.join("f"), "A1\n").unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["add", "f"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "-m", "commit A1"])
        .status()
        .unwrap();
    fs::write(repo_path.join("f"), "A2\n").unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["add", "f"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "-m", "commit A2"])
        .status()
        .unwrap();

    // Step B
    Command::new("git")
        .current_dir(&repo_path)
        .args(["checkout", "-b", "branch-b"])
        .status()
        .unwrap();
    fs::write(repo_path.join("f"), "B\n").unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["add", "f"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "-m", "commit B"])
        .status()
        .unwrap();

    (tmp, GitRepo::new(repo_path))
}

#[test]
fn test_managed_staircase_updates() {
    let (_tmp, repo) = setup_repo();
    let a2_oid = repo.resolve_ref("branch-a").unwrap();
    let a1_oid = repo.resolve_ref("branch-a~1").unwrap();
    let b_oid = repo.resolve_ref("branch-b").unwrap();

    let metadata = StaircaseMetadata {
        id: "test-managed".to_string(),
        name: "Managed".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "A".to_string(),
                cut: a2_oid.clone(),
                branch: Some("branch-a".to_string()),
            },
            Step {
                name: "B".to_string(),
                cut: b_oid.clone(),
                branch: Some("branch-b".to_string()),
            },
        ],
        verification_policy: None,
    };
    core::adopt(&repo, &metadata).unwrap();

    let rs = ResolvedStaircase::Managed(metadata);

    // Split Step A
    core::split(&repo, &rs, 0, &a1_oid, Some("A-part1")).unwrap();
    let rs = core::resolve_staircase(&repo, "test-managed", None)
        .unwrap()
        .unwrap();
    assert_eq!(rs.metadata().steps.len(), 3);
    assert_eq!(rs.metadata().steps[0].name, "A-part1");
    assert!(
        repo.resolve_ref(&format!("refs/staircase-state/{}/steps/A-part1", rs.metadata().id))
            .is_ok()
    );

    // Join Step A-part1 and A
    core::join(&repo, &rs, 0, 1).unwrap();
    let rs = core::resolve_staircase(&repo, "test-managed", None)
        .unwrap()
        .unwrap();
    assert_eq!(rs.metadata().steps.len(), 2);
    assert!(
        repo.resolve_ref(&format!("refs/staircase-state/{}/steps/A-part1", rs.metadata().id))
            .is_err()
    );

    // Restack (no-op here)
    core::restack(&repo, &rs).unwrap();
}

#[test]
fn test_implicit_staircase_updates() {
    let (_tmp, repo) = setup_repo();
    let a2_oid = repo.resolve_ref("branch-a").unwrap();
    let a1_oid = repo.resolve_ref("branch-a~1").unwrap();
    let b_oid = repo.resolve_ref("branch-b").unwrap();

    let metadata = StaircaseMetadata {
        id: "test-implicit".to_string(),
        name: "Implicit".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                name: "A".to_string(),
                cut: a2_oid.clone(),
                branch: Some("branch-a".to_string()),
            },
            Step {
                name: "B".to_string(),
                cut: b_oid.clone(),
                branch: Some("branch-b".to_string()),
            },
        ],
        verification_policy: None,
    };

    let rs = ResolvedStaircase::Implicit(metadata);

    // Split Step A
    core::split(&repo, &rs, 0, &a1_oid, Some("branch-a-part1")).unwrap();
    // For implicit, we need to re-resolve or just trust the side effects.
    // Actually core::split for implicit with new_step_name updates branches.
    assert!(repo.resolve_ref("branch-a-part1").is_ok());

    // Join (for implicit it might just update metadata if we adopt it)
    // The current implementation of join for implicit:
    // if removed_step.branch.is_some() { adopt(repo, &metadata)?; }
    // which makes it managed! That seems like a bug or at least weird behavior.
}

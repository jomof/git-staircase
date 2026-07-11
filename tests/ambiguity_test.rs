use git_staircase::GitRepo;
use git_staircase::core;
use git_staircase::model::StaircaseMetadata;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use uuid::Uuid;

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

#[test]
fn test_resolve_ambiguity_managed_vs_implicit() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    // Create an implicit staircase named 'feat'
    run_git(&path, &["checkout", "-b", "feat-1"]);
    let _feat1_oid = commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    let _feat2_oid = commit(&path, "feat2.txt", "2", "feat 2");

    // Create a managed staircase also named 'feat' (different steps to distinguish)
    run_git(&path, &["checkout", "main"]);
    run_git(&path, &["checkout", "-b", "other-1"]);
    let other1_oid = commit(&path, "other1.txt", "1", "other 1");

    let metadata = StaircaseMetadata {
        id: Uuid::new_v4().to_string(),
        name: "feat".to_string(),
        target: "main".to_string(),
        steps: vec![git_staircase::model::Step {
            name: "other-1".to_string(),
            cut: other1_oid,
            branch: Some("other-1".to_string()),
        }],
        verification_policy: None,
    };
    core::adopt(&repo, &metadata).unwrap();

    // Now 'feat' should be ambiguous
    let result = core::resolve_staircase(&repo, "feat", Some("main"));

    match result {
        Err(git_staircase::error::StaircaseError::Ambiguous(msg)) => {
            assert!(
                msg.contains("ambiguous"),
                "Error message should mention ambiguity: {}",
                msg
            );
        }
        Ok(Some(rs)) => {
            panic!("Expected Ambiguous error, but got {:?}", rs);
        }
        other => {
            panic!("Expected Ambiguous error, but got {:?}", other);
        }
    }
}

#[test]
fn test_resolve_ambiguity_multiple_implicit() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    // Create first implicit staircase named 'feat'
    run_git(&path, &["checkout", "main"]);
    run_git(&path, &["checkout", "-b", "feat-1"]);
    commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    commit(&path, "feat2.txt", "2", "feat 2");

    // Create second implicit staircase also named 'feat' (starting from a different branch)
    run_git(&path, &["checkout", "main"]);
    run_git(&path, &["checkout", "-b", "feat-alpha"]);
    commit(&path, "alpha.txt", "alpha", "feat alpha");
    run_git(&path, &["checkout", "-b", "feat-beta"]);
    commit(&path, "beta.txt", "beta", "feat beta");

    // Now 'feat' should be ambiguous
    let result = core::resolve_staircase(&repo, "feat", Some("main"));

    match result {
        Err(git_staircase::error::StaircaseError::Ambiguous(msg)) => {
            assert!(
                msg.contains("ambiguous"),
                "Error message should mention ambiguity: {}",
                msg
            );
        }
        Ok(Some(rs)) => {
            panic!("Expected Ambiguous error, but got {:?}", rs);
        }
        other => {
            panic!("Expected Ambiguous error, but got {:?}", other);
        }
    }
}

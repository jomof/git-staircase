mod common;
use common::*;
use git_staircase::core;
use git_staircase::model::StaircaseMetadata;
use git_staircase::*;
use uuid::Uuid;

#[test]
fn test_resolve_ambiguity_managed_vs_implicit() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    // Create an implicit staircase named 'feat'
    run_git(path, &["checkout", "-b", "feat-1"]);
    let _feat1_oid = commit(path, "feat1.txt", "1", "feat 1");
    run_git(path, &["checkout", "-b", "feat-2"]);
    let _feat2_oid = commit(path, "feat2.txt", "2", "feat 2");

    // Create a managed staircase also named 'feat' (different steps to distinguish)
    run_git(path, &["checkout", "main"]);
    run_git(path, &["checkout", "-b", "other-1"]);
    let other1_oid = commit(path, "other1.txt", "1", "other 1");

    let metadata = StaircaseMetadata {
        id: Uuid::new_v4().to_string(),
        name: "feat".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
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
        Err(StaircaseError::Ambiguous(msg)) => {
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
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    // Create first implicit staircase named 'feat'
    run_git(path, &["checkout", "main"]);
    run_git(path, &["checkout", "-b", "feat-1"]);
    commit(path, "feat1.txt", "1", "feat 1");
    run_git(path, &["checkout", "-b", "feat-2"]);
    commit(path, "feat2.txt", "2", "feat 2");

    // Create second implicit staircase also named 'feat' (starting from a different branch)
    run_git(path, &["checkout", "main"]);
    run_git(path, &["checkout", "-b", "feat-alpha"]);
    commit(path, "alpha.txt", "alpha", "feat alpha");
    run_git(path, &["checkout", "-b", "feat-beta"]);
    commit(path, "beta.txt", "beta", "feat beta");

    // Now 'feat' should be ambiguous
    let result = core::resolve_staircase(&repo, "feat", Some("main"));

    match result {
        Err(StaircaseError::Ambiguous(msg)) => {
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

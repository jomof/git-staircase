mod common;
use common::*;
use git_staircase::core;

#[test]
fn test_selector_ambiguity_with_git_revision() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // 1. Create a branch named 'auth' at the initial commit (already merged in 'main')
    // This Git revision will NOT denote a staircase relative to 'main'.
    let initial_oid = repo.resolve_ref("main").unwrap();
    run_git(dir, &["branch", "auth", &initial_oid]);

    // 2. Create and adopt a staircase named 'auth'
    run_git(dir, &["checkout", "main"]);
    run_git(dir, &["checkout", "-b", "feature/staircase"]);
    let _c2 = commit(dir, "feat.txt", "feat", "feat commit");

    let discoveries = core::discover(&repo, Some("main")).unwrap();
    let mut s = match &discoveries[0] {
        git_staircase::Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery"),
    };
    s.name = "auth".to_string();
    core::adopt(&repo, &s).unwrap();

    // ACT: Resolve the selector 'auth'
    let result = core::resolve_staircase(&repo, "auth", Some("main"));

    // ASSERT: Verify it fails with an ambiguity error
    match result {
        Err(git_staircase::error::StaircaseError::Ambiguous(msg)) => {
            println!("Ambiguity message:\n{}", msg);
            assert!(
                msg.contains("error: selector 'auth' is ambiguous"),
                "Message should have error prefix"
            );
            assert!(
                msg.contains("managed staircase:"),
                "Message should contain managed staircase"
            );
            assert!(
                msg.contains("Git revision:"),
                "Message should contain Git revision"
            );
        }
        Ok(res) => {
            panic!("Expected Ambiguous error, but got Ok({:?})", res);
        }
        Err(e) => {
            panic!("Expected Ambiguous error, but got {:?}", e);
        }
    }
}

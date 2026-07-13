use crate::common::*;
use git_staircase::core;

#[test]
fn test_selector_ambiguity_with_git_revision() {
    // ARRANGE
    let ctx = TestContext::new();

    // 1. Create a branch named 'auth' at the initial commit (already merged in 'main')
    // This Git revision will NOT denote a staircase relative to 'main'.
    let initial_oid = ctx.repo.resolve_ref("main").unwrap();
    ctx.run_git(&["branch", "auth", &initial_oid]);

    // 2. Create and adopt a staircase named 'auth'
    ctx.run_git(&["checkout", "main"]);
    ctx.run_git(&["checkout", "-b", "feature/staircase"]);
    ctx.commit("feat.txt", "feat", "feat commit");

    let discoveries = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    let mut s = match &discoveries[0] {
        git_staircase::Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery"),
    };
    s.name = "auth".to_string();
    core::adopt(&ctx.repo, &s).unwrap();

    // ACT: Resolve the selector 'auth'
    let result = core::resolve_staircase(&ctx.repo, "auth", Some("main"));

    // ASSERT: Verify it fails with an ambiguity error
    match result {
        Err(git_staircase::error::StaircaseError::Ambiguous(msg)) => {
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
        _ => panic!("Expected Ambiguous error, but got {:?}", result),
    }
}

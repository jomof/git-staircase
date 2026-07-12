mod common;
use common::*;
use git_staircase::*;

#[test]
fn test_restack_propagation() {
    // ARRANGE
    let ctx = TestContext::new();

    // 1. Repo already initialized with one commit by TestContext::new() (via setup_repo)

    // 2. Create a chain of 3 branches: s1 -> s2 -> s3
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("s1.txt", "s1", "s1 commit");

    ctx.run_git(&["checkout", "-b", "s2"]);
    let c2 = ctx.commit("s2.txt", "s2", "s2 commit");

    ctx.run_git(&["checkout", "-b", "s3"]);
    let c3 = ctx.commit("s3.txt", "s3", "s3 commit");

    // 3. Adopt as a staircase
    let sc = StaircaseMetadata {
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
            Step {
                id: String::new(),
                name: "s3".to_string(),
                cut: c3.clone(),
                branch: Some("s3".to_string()),
            },
        ],
        verification_policy: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();

    // 4. Modify s1 by rebasing it onto a new commit on main
    ctx.run_git(&["checkout", "main"]);
    ctx.commit("main2.txt", "main2", "main update");

    ctx.run_git(&["checkout", "s1"]);
    ctx.run_git(&["rebase", "main"]);
    let s1_new = ctx.run_git(&["rev-parse", "HEAD"]);

    let rs = git_staircase::core::resolve_staircase(&ctx.repo, "test", None)
        .unwrap()
        .unwrap();

    // ACT: Run restack
    git_staircase::core::restack(&ctx.repo, &rs).unwrap();

    // ASSERT: Verify results
    let s1_final = ctx.run_git(&["rev-parse", "s1"]);
    let s2_final = ctx.run_git(&["rev-parse", "s2"]);
    let s3_final = ctx.run_git(&["rev-parse", "s3"]);

    assert_eq!(s1_final, s1_new);

    // s2 should have been rebased onto s1_new
    assert!(ctx.repo.is_ancestor(&s1_final, &s2_final).unwrap());

    // s3 SHOULD have been rebased onto s2_new
    assert!(
        ctx.repo.is_ancestor(&s2_final, &s3_final).unwrap(),
        "s3 was not rebased onto s2"
    );
}

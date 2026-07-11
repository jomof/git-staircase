mod common;
use common::*;
use git_staircase::core::resolve_staircase;
use git_staircase::core::verification::verify;

#[test]
fn test_checkout_guard_detached_head() -> anyhow::Result<()> {
    // ARRANGE: Setup a git repo with two commits
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    let oid1 = run_git(repo_path, &["rev-parse", "HEAD"]);

    let oid2 = commit(repo_path, "file2", "content2", "commit 2");

    // Go to detached HEAD at commit 1
    run_git(repo_path, &["checkout", &oid1]);

    // Setup an implicit staircase to verify
    run_git(repo_path, &["branch", "-f", "main", &oid1]);
    run_git(repo_path, &["branch", "step1", &oid2]);

    let rs = resolve_staircase(&repo, "step1", Some("main"))?
        .expect("Staircase step1 not found");

    // ACT: Run verify (this will checkout oid2 and then try to restore)
    let _ = verify(&repo, &rs, None, None, None, None)?;

    // ASSERT: Check if we are back at oid1
    let current_oid = run_git(repo_path, &["rev-parse", "HEAD"]);

    assert_eq!(
        current_oid, oid1,
        "Should have restored to the original OID"
    );

    Ok(())
}

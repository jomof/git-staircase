mod common;
use common::*;

#[test]
fn test_git_repo_resolution_unification() {
    let ctx = TestContext::new();
    let path = &ctx.repo.workdir;

    // ARRANGE: Create a commit, a tag, and a branch
    let c1 = ctx.commit("file1.txt", "content1", "commit 1");
    run_git(path, &["tag", "tag1", &c1]);

    let c2 = ctx.commit("file2.txt", "content2", "commit 2");
    run_git(path, &["branch", "branch1", &c2]);

    // ACT: Resolve various types
    let res_tag = ctx.repo.resolve_commit("tag1").expect("resolve tag1");
    let res_branch = ctx.repo.resolve_commit("branch1").expect("resolve branch1");
    let res_short = ctx
        .repo
        .resolve_commit(&c1[..7])
        .expect("resolve short sha");
    let res_full = ctx.repo.resolve_commit(&c1).expect("resolve full sha");

    // ASSERT: Resolutions are correct
    assert_eq!(res_tag, c1);
    assert_eq!(res_branch, c2);
    assert_eq!(res_short, c1);
    assert_eq!(res_full, c1);

    // ACT: Check resolve_ref_opt
    assert_eq!(ctx.repo.resolve_ref_opt("tag1").unwrap(), Some(c1.clone()));
    assert_eq!(
        ctx.repo.resolve_ref_opt("branch1").unwrap(),
        Some(c2.clone())
    );
    assert_eq!(ctx.repo.resolve_ref_opt("non-existent").unwrap(), None);

    // ACT: Check resolve_symbolic_full_name
    assert_eq!(
        ctx.repo.resolve_symbolic_full_name("branch1").unwrap(),
        "refs/heads/branch1"
    );
    assert_eq!(
        ctx.repo.resolve_symbolic_full_name("tag1").unwrap(),
        "refs/tags/tag1"
    );
}

#[test]
fn test_git_repo_resolution_memoization() {
    let ctx = TestContext::new();
    let path = &ctx.repo.workdir;

    let c1 = ctx.commit("file1.txt", "content1", "commit 1");
    run_git(path, &["branch", "feat", &c1]);

    // Initially, memoizer should be empty for "feat"
    assert!(ctx.repo.memoizer.get_resolve_commit("feat").is_none());
    assert!(ctx.repo.memoizer.get_resolve_ref("feat").is_none());
    assert!(ctx.repo.memoizer.get_symbolic_name("feat").is_none());

    // ACT: Resolve commit
    let res = ctx.repo.resolve_commit("feat").unwrap();
    assert_eq!(res, c1);

    // ASSERT: Memoized after resolve_commit
    assert_eq!(
        ctx.repo.memoizer.get_resolve_commit("feat"),
        Some(c1.clone())
    );

    // ACT: Resolve ref
    let res_ref = ctx.repo.resolve_ref_opt("feat").unwrap();
    assert_eq!(res_ref, Some(c1.clone()));

    // ASSERT: Memoized after resolve_ref_opt
    assert_eq!(
        ctx.repo.memoizer.get_resolve_ref("feat"),
        Some(Some(c1.clone()))
    );

    // ACT: Resolve symbolic name
    let full_name = ctx.repo.resolve_symbolic_full_name("feat").unwrap();
    assert_eq!(full_name, "refs/heads/feat");

    // ASSERT: Memoized after resolve_symbolic_full_name
    assert_eq!(
        ctx.repo.memoizer.get_symbolic_name("feat"),
        Some("refs/heads/feat".to_string())
    );
}

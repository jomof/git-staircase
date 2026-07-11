mod common;
use common::*;
use git_staircase::core;
use git_staircase::ResolvedStaircase;
use git_staircase::Discovery;

#[test]
fn test_resolve_managed_by_internal_step_branch() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // 1. Create a staircase: A -> B -> C
    run_git(dir, &["checkout", "-b", "branch-a"]);
    let c1 = commit(dir, "a.txt", "a", "a");

    run_git(dir, &["checkout", "-b", "branch-b"]);
    let c2 = commit(dir, "b.txt", "b", "b");

    run_git(dir, &["checkout", "-b", "branch-c"]);
    let c3 = commit(dir, "c.txt", "c", "c");

    // 2. Adopt it to make it managed
    let discoveries = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(mut s) = discoveries[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "my-staircase".to_string();
    let managed_s = core::adopt(&repo, &s).unwrap();
    let uuid = managed_s.id.clone();

    // 3. Resolve by internal step branch name "branch-b"
    let resolved = core::resolve_staircase(&repo, "branch-b", Some("main"))
        .unwrap()
        .expect("Should resolve");

    // ASSERT: It should resolve to the Managed staircase, not an implicit one,
    // and it should be the full staircase (3 steps), not truncated.
    assert!(resolved.is_managed());
    if let ResolvedStaircase::Managed(meta) = resolved {
        assert_eq!(meta.id, uuid);
        assert_eq!(meta.name, "my-staircase");
        assert_eq!(meta.steps.len(), 3);
        assert_eq!(meta.steps[0].cut, c1);
        assert_eq!(meta.steps[1].cut, c2);
        assert_eq!(meta.steps[2].cut, c3);
    } else {
        panic!("Expected Managed variant");
    }

    // 4. Resolve by internal step cut OID "c2"
    let resolved_by_oid = core::resolve_staircase(&repo, &c2, Some("main"))
        .unwrap()
        .expect("Should resolve by OID");

    assert!(resolved_by_oid.is_managed());
    assert_eq!(resolved_by_oid.metadata().id, uuid);
}

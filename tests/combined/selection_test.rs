
use crate::common::*;
use git_staircase::core;

#[test]
fn test_resolve_explicit_staircase() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(path, "feat1.txt", "1", "feat 1");
    run_git(path, &["checkout", "-b", "feat-2"]);
    let c2 = commit(path, "feat2.txt", "2", "feat 2");

    let rs = core::resolve_explicit_staircase(
        &repo,
        &["feat-1".to_string(), "feat-2".to_string()],
        Some("main"),
    )
    .unwrap();

    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    assert_eq!(metadata.steps.len(), 2);
    assert_eq!(metadata.steps[0].name, "feat-1");
    assert_eq!(metadata.steps[0].cut, c1);
    assert_eq!(metadata.steps[1].name, "feat-2");
    assert_eq!(metadata.steps[1].cut, c2);
    assert_eq!(metadata.target, "refs/heads/main");
}

#[test]
fn test_resolve_by_oid_sub_staircase() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(path, "feat1.txt", "1", "feat 1");
    run_git(path, &["checkout", "-b", "feat-2"]);
    let _c2 = commit(path, "feat2.txt", "2", "feat 2");

    // Resolve by c1 OID (which is the cut of feat-1)
    let rs = core::resolve_staircase(&repo, &c1, Some("main"))
        .unwrap()
        .expect("Should resolve by OID");

    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    assert_eq!(metadata.steps.len(), 1);
    assert_eq!(metadata.steps[0].name, "feat-1");
    assert_eq!(metadata.steps[0].cut, c1);
}

#[test]
fn test_resolve_from_ambiguous_family() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    // Create a fork (ambiguous family):
    // main -> step1 -> step2a
    //              -> step2b
    run_git(path, &["checkout", "-b", "step1"]);
    let c1 = commit(path, "file1.txt", "1", "commit 1");

    run_git(path, &["checkout", "-b", "step2a"]);
    let c2a = commit(path, "file2a.txt", "2a", "commit 2a");

    run_git(path, &["checkout", "step1"]);
    run_git(path, &["checkout", "-b", "step2b"]);
    let _c2b = commit(path, "file2b.txt", "2b", "commit 2b");

    // Resolve by OID c2a
    let rs = core::resolve_staircase(&repo, &c2a, Some("main"))
        .unwrap()
        .expect("Should resolve from family by OID");

    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    // Path should be: step1 -> step2a
    assert_eq!(metadata.steps.len(), 2);
    assert_eq!(metadata.steps[0].name, "step1");
    assert_eq!(metadata.steps[0].cut, c1);
    assert_eq!(metadata.steps[1].name, "step2a");
    assert_eq!(metadata.steps[1].cut, c2a);
}

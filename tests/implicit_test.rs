mod common;
use common::*;
use git_staircase::IdentityKind;
use git_staircase::core;

#[test]
fn test_implicit_staircase_operations() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create linear chain: main -> feature/auth-core -> feature/auth-ui
    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let name = "feature/auth";

    // 1. resolve_staircase
    let rs = core::resolve_staircase(&repo, name, None)
        .unwrap()
        .expect("Should find implicit staircase");
    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    assert_eq!(metadata.name, name);
    assert_eq!(metadata.steps.len(), 2);

    // 2. get_status_metadata
    let status = core::get_status_metadata(&repo, metadata.clone(), !rs.is_managed()).unwrap();
    assert!(status.is_clean);
    assert_eq!(status.steps[0].actual_oid, Some(c1));
    assert_eq!(status.steps[1].actual_oid, Some(c2));

    // 3. compute_identity
    let id_nominal = core::compute_identity(&repo, &rs, IdentityKind::Nominal).unwrap();
    assert_eq!(id_nominal, name);

    let id_revision = core::compute_identity(&repo, &rs, IdentityKind::Revision).unwrap();
    assert!(!id_revision.is_empty());

    // 4. verify (just check it doesn't crash, since we don't have policy)
    let results = core::verify(&repo, &rs, None, None, Some(false), Some(true)).unwrap();
    assert_eq!(results.len(), 2);
}

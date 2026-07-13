mod common;
use common::*;
use git_staircase::Discovery;
use git_staircase::core;

#[test]
fn test_staircase_storage_alignment() {
    // ARRANGE: Create and adopt a staircase
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();

    let s = core::adopt(&repo, &s).unwrap();

    // ACT: Resolve refs/staircases/auth
    let ref_name = "refs/staircases/auth";
    let oid = repo
        .resolve_ref(ref_name)
        .expect("refs/staircases/auth should exist");

    // ASSERT: Verify it points to a tree record object (Spec Addendum I)
    let content = repo.run(&["cat-file", "-p", &format!("{}:structure", ref_name)]).unwrap();
    assert!(
        content.starts_with("git-staircase-descriptor 1\n"),
        "Descriptor should have canonical header"
    );

    // Also verify refs/staircases/auth is a tree type
    let obj_type = repo.run(&["cat-file", "-t", &oid]).unwrap();
    assert_eq!(obj_type.trim(), "tree");

    // Verify old fragmented structure is GONE
    let meta_ref = format!("refs/staircases/{}/meta", s.id);
    assert!(
        repo.resolve_ref(&meta_ref).is_err(),
        "Old meta ref should not exist"
    );

    // ARRANGE: Update a step
    run_git(dir, &["checkout", "feature/auth-ui"]);
    let c2_new = commit(dir, "file2_mod.txt", "2 mod", "commit 2 mod");

    let status = core::get_status(&repo, &s.id).unwrap();
    let mut metadata = status.metadata.clone();
    metadata.steps[1].cut = c2_new.clone();

    core::adopt(&repo, &metadata).unwrap();

    // ACT: Get the new revision ID
    let new_oid = repo.resolve_ref(ref_name).unwrap();

    // ASSERT: Verify the ID matches the new record OID
    assert_ne!(oid, new_oid, "Revision ID should change after update");
    let new_content = repo.run(&["cat-file", "-p", &format!("{}:structure", ref_name)]).unwrap();
    assert!(
        new_content.contains(&c2_new),
        "New descriptor should contain new cut OID"
    );

    // Verify it still resolves via name
    let found = core::find_by_name(&repo, "auth")
        .unwrap()
        .expect("Should find auth by name");
    assert_eq!(found.id, s.id);
}

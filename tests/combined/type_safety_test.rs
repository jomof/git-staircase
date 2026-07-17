
use crate::common::*;
use std::process::Command;

#[test]
fn test_resolve_commit_with_blob_tag_should_fail() {
    // ARRANGE
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create a blob and a tag pointing to it
    let blob_oid = repo
        .run_with_stdin(&["hash-object", "-w", "--stdin"], "hello world")
        .unwrap();
    let blob_oid = blob_oid.trim();
    Command::new("git")
        .current_dir(dir)
        .args(["tag", "blob-tag", &blob_oid])
        .status()
        .unwrap();

    // ACT
    let result = repo.resolve_commit("blob-tag");

    // ASSERT
    // According to spec, this SHOULD fail because it's not a commit.
    assert!(
        result.is_err(),
        "Expected resolve_commit to fail for a blob tag, but it succeeded with OID: {:?}",
        result.ok()
    );
}

#[test]
fn test_resolve_commit_with_commit_should_succeed() {
    // ARRANGE
    let (_tmp, repo) = setup_repo();
    let _dir = &repo.workdir;

    let oid = repo.resolve_commit("main").unwrap();

    // ACT
    let result = repo.resolve_commit("main");

    // ASSERT
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), oid);
}

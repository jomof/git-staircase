mod common;
use common::*;
use git_staircase::core;
use std::fs;

#[test]
fn test_corrupted_journal_dos() {
    // ARRANGE
    let ctx = TestContext::new();
    let staircase_dir = ctx.repo.common_dir().unwrap().join("staircase");
    let journals_dir = staircase_dir.join("journals");
    fs::create_dir_all(&journals_dir).unwrap();

    // Create a corrupted (malformed) journal file
    let bad_journal = journals_dir.join("corrupted.json");
    fs::write(&bad_journal, "{ \"invalid\": json }").unwrap();

    // ACT & ASSERT
    // This should succeed by ignoring the corrupted file or treating it as "no active operation"
    // safely, but currently it fails and bricks the tool.
    core::operation::ensure_no_active(&ctx.repo)
        .expect("Should succeed despite corrupted journal file");

    // Similarly, abort should be able to proceed or at least not fail on the malformed JSON
    // so that the user can clear the state.
    core::operation::abort_active(&ctx.repo)
        .expect("Should be able to abort or report no active operation");
}

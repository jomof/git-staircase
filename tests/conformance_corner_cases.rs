mod common;
use common::*;
use std::fs;

#[test]
#[ignore]
fn empty_step_is_rejected_before_mutation() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // ARRANGE: Create a repository with three refs: step1, step2, step3.
    // main -> A (step1) -> B (step2) -> C (step3)
    // But set step2 == step1, so step 2 is empty.
    run_git(dir, &["checkout", "-b", "feature/auth-1"]);
    commit(dir, "file1.txt", "1", "commit 1");
    let oid1 = get_head_oid(dir);

    run_git(dir, &["checkout", "-b", "feature/auth-2"]);
    // No new commit, same OID as auth-1
    let oid2 = get_head_oid(dir);
    assert_eq!(oid1, oid2);

    run_git(dir, &["checkout", "-b", "feature/auth-3"]);
    commit(dir, "file3.txt", "3", "commit 3");

    // ACT: Attempt to list or inspect this as a multi-step staircase.
    let (_success, stdout, _stderr) = run_staircase(dir, &["list", "--implicit"]);

    // ASSERT: The command should fail or exclude the candidate, reporting that empty steps are forbidden.
    // If it's normalized correctly, it should either be rejected or the empty step should be removed.
    // The spec says "reject such candidates before they are promoted to canonical implicit staircases".
    assert!(
        !stdout.contains("feature/auth"),
        "Staircase with empty step should be excluded from discovery. Output: {}",
        stdout
    );
}

#[test]
fn integration_branch_at_anchor_is_not_discovered() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // ARRANGE: Initialize a repo with a 'main' branch and a 'feature' branch pointing to the ancestor of 'main'.
    // main is at init.txt
    let ancestor_oid = get_head_oid(dir);

    // Add a new commit to main
    commit(dir, "main_only.txt", "main content", "main commit");
    let main_oid = get_head_oid(dir);
    assert_ne!(ancestor_oid, main_oid);

    // feature is at the ancestor
    run_git(dir, &["update-ref", "refs/heads/feature", &ancestor_oid]);

    // ACT: Run 'git staircase list' with 'main' as the integration anchor.
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--implicit", "--onto", "main"]);

    // ASSERT: The output should indicate 'No staircases' because 'feature' is an ancestor of the anchor.
    assert!(success, "list failed: {}", stderr);
    assert!(
        !stdout.contains("feature"),
        "Ancestor branch of anchor should not be discovered. Output: {}",
        stdout
    );
}

#[test]
fn ignored_files_do_not_make_default_draft_dirty() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // ARRANGE: Create a clean repository and add a file matching a pattern in '.gitignore'.
    fs::write(dir.join(".gitignore"), "ignored.txt\n").unwrap();
    run_git(dir, &["add", ".gitignore"]);
    run_git(dir, &["commit", "-m", "add gitignore"]);

    fs::write(dir.join("ignored.txt"), "some content").unwrap();

    // ACT: Execute a command that requires a clean worktree disposition, such as 'git staircase archive'.
    // Note: archive might not be implemented, let's try 'draft status' and check if it's considered clean.
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status"]);

    // ASSERT: The command should succeed without complaining about dirty files.
    assert!(success, "draft status failed: {}", stderr);
    assert!(
        !stdout.contains("untracked: 1 paths"),
        "Ignored file should not be reported as untracked. Output: {}",
        stdout
    );
}

#[test]
#[ignore]
fn untracked_files_require_explicit_inclusion() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // ARRANGE: Create a clean repository and add a new file that is untracked (and not ignored).
    fs::write(dir.join("untracked.txt"), "untracked content").unwrap();

    // ACT: Run 'git staircase list' or attempt to create a draft.
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status"]);
    assert!(success, "draft status failed: {}", stderr);

    // ASSERT: Verify that the untracked file is reported as a separate concern and is NOT included in the draft OID.
    assert!(
        stdout.contains("untracked: 1 paths"),
        "Untracked file should be reported separately. Output: {}",
        stdout
    );

    // ACT: Create a snapshot and verify it does NOT include the untracked file
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "snapshot"]);
    assert!(success, "snapshot failed: {}", stderr);

    let snapshot_id = stdout
        .split_whitespace()
        .find(|w| w.len() == 36 && w.contains('-'))
        .expect("Snapshot UUID not found");

    // Clear worktree
    run_git(dir, &["clean", "-fd"]);
    assert!(!dir.join("untracked.txt").exists());

    // Restore snapshot
    let (success, _, stderr) = run_staircase(dir, &["draft", "restore", snapshot_id]);
    assert!(success, "restore failed: {}", stderr);

    // ASSERT: Untracked file should NOT be restored if it wasn't included in the snapshot
    assert!(
        !dir.join("untracked.txt").exists(),
        "Untracked file should NOT be included in snapshot/restore by default"
    );
}

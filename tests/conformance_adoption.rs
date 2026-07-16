mod common;
use common::*;

#[test]
fn observation_never_adopts() {
    // ARRANGE
    // Create an unmanaged staircase (e.g., branch 'feature' ahead of 'main').
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature-1"]);
    commit(dir, "f1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "feature-2"]);
    commit(dir, "f2.txt", "2", "commit 2");

    // ACT
    // Run 'git-staircase show feature-2'.
    let (success, _stdout, stderr) = run_staircase(dir, &["show", "feature-2"]);
    assert!(success, "show command failed: {}", stderr);

    // ASSERT
    // Verify that 'git for-each-ref refs/staircases/' returns no results, proving no metadata was persisted.
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        refs.is_empty(),
        "Observation triggered adoption! Refs found: {}",
        refs
    );

    // ACT: Run 'git-staircase status feature-2'.
    let (success, _stdout, stderr) = run_staircase(dir, &["status", "feature-2"]);
    assert!(success, "status command failed: {}", stderr);

    // ASSERT
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        refs.is_empty(),
        "Status triggered adoption! Refs found: {}",
        refs
    );

    // ACT: Run 'git-staircase list --implicit'.
    let (success, _stdout, stderr) = run_staircase(dir, &["list", "--implicit"]);
    assert!(success, "list command failed: {}", stderr);

    // ASSERT
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        refs.is_empty(),
        "List triggered adoption! Refs found: {}",
        refs
    );

    // ACT: Run 'git-staircase show --ids feature-2'.
    // This is the one that is expected to FAIL and trigger adoption currently.
    let (success, _stdout, stderr) = run_staircase(dir, &["show", "feature-2", "--ids"]);
    assert!(success, "show --ids command failed: {}", stderr);

    // ASSERT
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        refs.is_empty(),
        "Show --ids triggered adoption! Refs found: {}",
        refs
    );
}

#[test]
fn revision_identity_remains_implicit_but_stable_identity_adopts() {
    // 1. ARRANGE: Create a 2-step staircase with no stable identifiers (no Change-Ids).
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();
    let anchor = run_git(dir, &["rev-parse", "main"]);

    run_git(dir, &["checkout", "-b", "feat"]);
    let c1 = commit(dir, "f1.txt", "1", "commit 1");
    run_git(dir, &["branch", "feat-1", &c1]);
    let c2 = commit(dir, "f2.txt", "2", "commit 2");
    run_git(dir, &["branch", "feat-2", &c2]);

    // 2. ACT: Run `git-staircase show` and `git-staircase list --implicit`.
    let (ok1, _, stderr1) = run_staircase(dir, &["show", "feat-2", "--onto", &anchor]);
    let (ok2, _, stderr2) = run_staircase(dir, &["list", "--implicit", "--onto", &anchor]);

    // 3. ASSERT: Verify no `refs/staircases/` were created in the repository.
    assert!(ok1, "show failed: {}", stderr1);
    assert!(ok2, "list failed: {}", stderr2);
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        refs.is_empty(),
        "Observation should not trigger adoption. Found refs: {}",
        refs
    );

    // 4. ACT: Add a `Change-Id` to one of the commits and run `git-staircase status`.
    run_git(
        dir,
        &[
            "commit",
            "--amend",
            "-m",
            "commit 2\n\nChange-Id: I1234567890abcdef1234567890abcdef12345678",
        ],
    );
    run_git(dir, &["branch", "-f", "feat-2", "HEAD"]);
    let (ok3, _, stderr3) = run_staircase(dir, &["status", "feat-2", "--onto", &anchor]);

    // 5. ASSERT: Verify still no adoption (observation only).
    assert!(ok3, "status failed: {}", stderr3);
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        refs.is_empty(),
        "Status with stable ID should not trigger adoption. Found refs: {}",
        refs
    );

    // 6. ACT: Run `git-staircase join` on the steps.
    let (ok4, _, stderr4) = run_staircase(
        dir,
        &[
            "join",
            "feat-2",
            "--step",
            "2",
            "--step2",
            "1",
            "--onto",
            &anchor,
            "--delete-boundary-ref",
        ],
    );
    assert!(ok4, "join failed: {}", stderr4);

    // 7. ASSERT: Verify adoption occurred and `refs/staircases/` now exists.
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        !refs.is_empty(),
        "Mutation (join) with stable ID should trigger adoption."
    );
}

#[test]
fn append_adopts_only_for_durable_association() {
    // 1. ARRANGE: Create an implicit (unmanaged) staircase.
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();
    let anchor = run_git(dir, &["rev-parse", "main"]);

    run_git(dir, &["checkout", "-b", "feat"]);
    commit(dir, "f1.txt", "1", "commit 1");

    // 2. ACT: Add a new commit to the tip of the branch and run `git-staircase show`.
    commit(dir, "f2.txt", "2", "commit 2");
    let (ok, _, stderr) = run_staircase(dir, &["show", "feat", "--onto", &anchor]);
    assert!(ok, "show failed: {}", stderr);

    // 3. ASSERT: Verify no adoption occurred (no `refs/staircases/`).
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        refs.is_empty(),
        "Appending commits should not trigger adoption via show."
    );

    // 4. ACT: Run a mutation like `git-staircase append`.
    // First, create a commit NOT on the branch so we can append it.
    let head = get_head_oid(dir);
    commit(dir, "f3.txt", "3", "commit 3");
    let c3 = get_head_oid(dir);
    run_git(dir, &["reset", "--hard", &head]);

    let (ok, _, stderr) = run_staircase(
        dir,
        &[
            "append",
            "feat",
            "--commits",
            &format!("{}..{}", head, c3),
            "--onto",
            &anchor,
        ],
    );
    assert!(ok, "append failed: {}", stderr);

    // 5. ASSERT: Verify adoption DID NOT occur (no stable ID).
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(refs.is_empty(), "Append without stable ID should not trigger adoption.");
}

#[test]
fn new_step_adopts_only_for_metadata_only_cut() {
    // 1. ARRANGE: Create an implicit staircase with a single step containing multiple commits.
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();
    let anchor = run_git(dir, &["rev-parse", "main"]);

    run_git(dir, &["checkout", "-b", "feat"]);
    let c1 = commit(dir, "f1.txt", "1", "commit 1");
    let _c2 = commit(dir, "f2.txt", "2", "commit 2");

    // 2. ACT: Perform a `split` that introduces a new step boundary between existing commits without creating new objects.
    let (ok, _, stderr) = run_staircase(
        dir,
        &[
            "split", "feat", "--step", "1", "--at", &c1, "--onto", &anchor, "--no-ref",
        ],
    );
    assert!(ok, "split failed: {}", stderr);

    // 3. ASSERT: Verify that the staircase is now adopted (managed).
    let refs = run_git(dir, &["for-each-ref", "refs/staircases/"]);
    assert!(
        !refs.is_empty(),
        "Metadata-only cut (split) should trigger adoption."
    );
}

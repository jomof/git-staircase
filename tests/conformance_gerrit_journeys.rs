mod common;

use common::*;

#[test]
fn journey_1_plan_rejects_missing_change_id_without_mutation() {
    // ARRANGE: Setup a repository with a Gerrit remote.
    let context = TestContext::new();

    // Setup Gerrit-like remote
    let remote_tmp = tempfile::TempDir::new().unwrap();
    let remote_path = remote_tmp.path();
    run_git(remote_path, &["init", "-b", "main"]);
    run_git(
        remote_path,
        &["config", "receive.denyCurrentBranch", "ignore"],
    );

    context.run_git(&["remote", "add", "origin", &remote_path.to_string_lossy()]);
    context.commit("base.txt", "base", "initial");
    context.run_git(&["push", "origin", "main"]);

    // Configure Gerrit host/project so discovery works even without .repo
    context.run_git(&["config", "gerrit.host", "gerrit.example.com"]);
    context.run_git(&["config", "gerrit.project", "test-project"]);

    // Create a commit without Change-Id
    context.commit("file.txt", "content", "Step 1 (no Change-Id)");
    context.run_git(&["branch", "topic"]);

    // ACT: Attempt to plan a review
    let (success, stdout, stderr) = context.run_staircase(&["review", "plan", "topic"]);

    // ASSERT: Operation fails with a clear error and no records are created.
    // Spec 13.4: "If required review commits lack Change-Ids, default `review create` fails before record mutation."
    // And the violation says detect during planning phase and reject.
    assert!(
        !success,
        "review plan should have failed due to missing Change-Id\nSTDOUT: {}\nSTDERR: {}",
        stdout, stderr
    );
    assert!(
        stderr.to_lowercase().contains("missing") || stdout.to_lowercase().contains("missing"),
        "Error message should mention missing Change-Id\nSTDOUT: {}\nSTDERR: {}",
        stdout,
        stderr
    );

    // Verify no records are created (refs/staircases/ should be empty)
    let records = context.run_git(&["for-each-ref", "refs/staircases/"]);
    assert!(
        records.trim().is_empty(),
        "No staircase records should have been created: {}",
        records
    );
}

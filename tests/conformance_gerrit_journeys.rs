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

#[test]
fn journey_1_discovers_new_review_stack() {
    // ARRANGE: Create a repository with a Gerrit remote and multiple commits, each with a unique Change-Id.
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

    // Configure Gerrit host/project
    context.run_git(&["config", "gerrit.host", "gerrit.example.com"]);
    context.run_git(&["config", "gerrit.project", "test-project"]);

    // Create a topic branch from main
    context.run_git(&["checkout", "-b", "topic"]);

    // Create three commits with Change-Id footers on the topic branch
    let c1 = context.commit(
        "file1.txt",
        "1",
        "Step 1\n\nChange-Id: I1111111111111111111111111111111111111111",
    );
    let c2 = context.commit(
        "file2.txt",
        "2",
        "Step 2\n\nChange-Id: I2222222222222222222222222222222222222222",
    );
    let c3 = context.commit(
        "file3.txt",
        "3",
        "Step 3\n\nChange-Id: I3333333333333333333333333333333333333333",
    );

    // ACT: Run git-staircase list --porcelain
    let (success, stdout, stderr) = context.run_staircase(&["list", "--porcelain"]);

    // ASSERT: Verify that the commits are correctly identified as a review stack.
    assert!(success, "git-staircase list failed: {}", stderr);
    assert!(
        stdout.contains("topic"),
        "Staircase 'topic' should be listed\nSTDOUT: {}",
        stdout
    );

    // We want to verify that it has 3 steps.
    let (success, stdout, stderr) = context.run_staircase(&["show", "topic", "--json"]);
    assert!(success, "git-staircase show failed: {}", stderr);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let steps = json["steps"].as_array().expect("steps should be an array");

    // Diagnostic failure: if this fails, it means Gerrit discovery didn't find the intermediate Change-Id commits as steps.
    assert_eq!(
        steps.len(),
        3,
        "Should have 3 steps (one for each Change-Id commit), found: {}. Steps: {:?}",
        steps.len(),
        steps
    );

    // Verify that Change-Id values are preserved as step identities
    assert_eq!(
        steps[0]["id"], "I1111111111111111111111111111111111111111",
        "Step 1 identity should match Change-Id"
    );
    assert_eq!(
        steps[1]["id"], "I2222222222222222222222222222222222222222",
        "Step 2 identity should match Change-Id"
    );
    assert_eq!(
        steps[2]["id"], "I3333333333333333333333333333333333333333",
        "Step 3 identity should match Change-Id"
    );

    assert_eq!(steps[0]["cut"], c1, "Step 1 cut should match commit 1");
    assert_eq!(steps[1]["cut"], c2, "Step 2 cut should match commit 2");
    assert_eq!(steps[2]["cut"], c3, "Step 3 cut should match commit 3");
}

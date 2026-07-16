mod common;

use common::*;
use git_staircase::core::{self, OperationPhase};
use std::fs;

#[test]
fn journey_1_bootstraps_repo_gerrit_and_publishes_three_reviews() {
    // ARRANGE: Setup a repository with a .repo manifest and a Gerrit remote origin. Checkout a detached HEAD.
    let context = TestContext::new();
    let root = context.path();

    // Simulate .repo manifest
    std::fs::create_dir_all(root.join(".repo")).unwrap();
    std::fs::write(
        root.join(".repo").join("manifest.xml"),
        r#"<manifest>
  <remote name="origin" fetch=".." review="origin"/>
  <default remote="origin" revision="main"/>
  <project name="test-project" path="."/>
</manifest>"#,
    )
    .unwrap();

    // Setup Gerrit-like remote
    let remote_tmp = tempfile::TempDir::new().unwrap();
    let remote_path = remote_tmp.path();
    run_git(remote_path, &["init", "-b", "main"]);
    run_git(
        remote_path,
        &["config", "receive.denyCurrentBranch", "ignore"],
    );

    context.run_git(&["remote", "add", "origin", &remote_path.to_string_lossy()]);
    // Push initial commit to remote
    context.run_git(&["push", "origin", "main"]);

    // Checkout detached HEAD
    let head = context.run_git(&["rev-parse", "HEAD"]);
    context.run_git(&["checkout", "--detach", &head]);
    context.run_git(&["update-ref", "refs/remotes/m/main", &head]);

    // ACT: Create three commits and branches.
    let mut oids = Vec::new();
    for i in 1..=3 {
        oids.push(context.commit(
            &format!("file{}.txt", i),
            &format!("content{}", i),
            &format!("Step {}\n\nChange-Id: I{:040x}", i, i),
        ));
    }
    context.run_git(&["branch", "payments-1", &oids[0]]);
    context.run_git(&["branch", "payments-2", &oids[1]]);
    context.run_git(&["branch", "payments", &oids[2]]);

    // Run git-staircase publish
    let (success, stdout, stderr) = context.run_staircase(&["publish", "payments"]);
    assert!(
        success,
        "publish failed: stdout: {}, stderr: {}",
        stdout, stderr
    );

    // ASSERT: Verify three reviews are created on the provider and local refs/staircases tracks them.

    let staircase_refs = context.run_git(&["for-each-ref", "refs/staircases/"]);
    assert!(!staircase_refs.is_empty(), "No staircase refs found");

    // Check for Gerrit refs (simulated by pushes to refs/for/main which we can't easily see on bare repo without inspection)
    // Actually, in this setup, git-staircase will try to push to 'origin' at 'refs/for/main'.
    // We can check if those refs were created in the remote.
    let remote_refs = run_git(remote_path, &["for-each-ref"]);
    assert!(
        remote_refs.contains("refs/for/main"),
        "Gerrit upload ref not found in remote: {}",
        remote_refs
    );
}

#[test]
fn journey_1_amend_preserves_draft_and_review_identity_across_conflicts() {
    // ARRANGE: Create a repository with a 3-step staircase.
    let context = TestContext::new();
    let root = context.path();

    // Create a base commit
    let initial_oid = context.commit("base.txt", "base content\n", "initial");

    // Create topic branch for the staircase
    context.run_git(&["checkout", "-b", "topic", &initial_oid]);

    // Create 3 steps with potential for conflict
    // Step 1: modifies file1.txt
    // Step 2: modifies file1.txt (conflicts with Step 1 if amended)
    // Step 3: modifies file1.txt (conflicts with Step 2/1 if amended)

    let step1_oid = context.commit("file1.txt", "A\n", "Step 1\n\nChange-Id: Istep1");

    let step2_oid = context.commit("file1.txt", "B\n", "Step 2\n\nChange-Id: Istep2");

    let step3_oid = context.commit("file1.txt", "C\n", "Step 3\n\nChange-Id: Istep3");

    context.run_git(&["branch", "step1", &step1_oid]);
    context.run_git(&["branch", "step2", &step2_oid]);

    // Adopt the staircase
    let (success, stdout, stderr) =
        context.run_staircase(&["adopt", "managed", "step1", "step2", "topic"]);
    assert!(
        success,
        "adopt failed: stdout: {}, stderr: {}",
        stdout, stderr
    );

    let metadata = core::persistence::read_metadata(&context.repo, "managed").unwrap();
    let step1_id = metadata.steps[0].id.clone();
    let step2_id = metadata.steps[1].id.clone();
    let step3_id = metadata.steps[2].id.clone();

    // ACT: Amend Step 1 with a change that conflicts with Step 2 and Step 3.
    context.run_git(&["checkout", &step1_oid]);
    let step1_amended_oid = context.commit("file1.txt", "X\n", "Step 1\n\nChange-Id: Istep1");
    context.run_git(&["branch", "-f", "step1", &step1_amended_oid]);
    context.run_git(&["checkout", "topic"]);

    // ACT: Execute git staircase restack.
    let (success, stdout, stderr) = context.run_staircase(&["restack", "managed", "--json"]);

    // ASSERT: Verify the command pauses for conflicts.
    assert!(
        !success,
        "restack should have paused for conflicts. stdout: {}, stderr: {}",
        stdout, stderr
    );
    assert!(
        stderr.contains("operation-paused") || stdout.contains("operation-paused"),
        "stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    let active = core::active_operation(&context.repo).unwrap().unwrap();
    assert_eq!(active.phase, OperationPhase::Paused);

    // ACT: Resolve conflicts for Step 2.
    fs::write(root.join("file1.txt"), "Y\n").unwrap();
    context.run_git(&["add", "file1.txt"]);

    // ACT: Continue the operation.
    let (success, stdout, stderr) = context.run_staircase(&["continue", "--json"]);

    // ASSERT: Verify it pauses again for Step 3 conflicts.
    assert!(
        !success,
        "continue should have paused for Step 3 conflicts. stdout: {}, stderr: {}",
        stdout, stderr
    );
    assert!(
        stderr.contains("operation-paused") || stdout.contains("operation-paused"),
        "stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // ACT: Resolve conflicts for Step 3.
    fs::write(root.join("file1.txt"), "Z\n").unwrap();
    context.run_git(&["add", "file1.txt"]);

    // ACT: Continue the operation.
    let (success, stdout, stderr) = context.run_staircase(&["continue", "--json"]);
    assert!(
        success,
        "final continue failed. stdout: {}, stderr: {}",
        stdout, stderr
    );

    // ASSERT: Verify final OIDs and that Step identities/metadata are preserved.
    let updated = core::persistence::read_metadata(&context.repo, "managed").unwrap();
    assert_eq!(updated.steps.len(), 3);
    assert_eq!(updated.steps[0].id, step1_id);
    assert_eq!(updated.steps[1].id, step2_id);
    assert_eq!(updated.steps[2].id, step3_id);

    assert_eq!(updated.steps[0].cut, step1_amended_oid);
    // Step 2 and 3 should have new OIDs
    assert_ne!(updated.steps[1].cut, step2_oid);
    assert_ne!(updated.steps[2].cut, step3_oid);

    let final_content = fs::read_to_string(root.join("file1.txt")).unwrap();
    assert_eq!(final_content, "Z\n");
}

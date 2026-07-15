mod common;

use common::*;
use std::process::Command;

#[test]
fn test_delete_implicit_staircase() {
    let context = TestContext::new();

    // Create an implicit staircase
    context.run_git(&["checkout", "-b", "feature-1"]);
    context.commit("f1.txt", "f1", "f1");
    context.run_git(&["checkout", "-b", "feature-2"]);
    context.commit("f2.txt", "f2", "f2");

    // 1. Try to delete it without --delete-materializing-refs -> Should fail as per spec
    let (success, stdout, stderr) = context.run_staircase(&["delete", "feature-2"]);
    assert!(
        !success,
        "Delete command should fail for implicit staircase without flag: {}",
        stdout
    );
    assert!(
        stderr.contains("use --delete-materializing-refs"),
        "Error message should suggest flag: {}",
        stderr
    );

    // Verify branches are still there
    assert!(
        !context
            .run_git(&["rev-parse", "--verify", "feature-1"])
            .is_empty()
    );
    assert!(
        !context
            .run_git(&["rev-parse", "--verify", "feature-2"])
            .is_empty()
    );

    // 2. Now delete with --delete-materializing-refs -> Should succeed
    let (success, _stdout, stderr) =
        context.run_staircase(&["delete", "feature-2", "--delete-materializing-refs"]);
    assert!(success, "Delete failed: {}", stderr);

    // Verify branches are gone
    let f1_check = Command::new("git")
        .current_dir(context.path())
        .args(["rev-parse", "--verify", "feature-1"])
        .output()
        .unwrap();
    assert!(!f1_check.status.success(), "feature-1 should be deleted");

    let f2_check = Command::new("git")
        .current_dir(context.path())
        .args(["rev-parse", "--verify", "feature-2"])
        .output()
        .unwrap();
    assert!(!f2_check.status.success(), "feature-2 should be deleted");
}

#[test]
fn test_delete_managed_staircase() {
    let context = TestContext::new();

    // Create and adopt a staircase
    context.run_git(&["checkout", "-b", "managed-1"]);
    context.commit("m1.txt", "m1", "m1");
    context.run_git(&["checkout", "-b", "managed-tip"]);
    context.commit("m2.txt", "m2", "m2");

    let (success, _, _) = context.run_staircase(&["adopt", "managed", "managed-1", "managed-tip"]);
    assert!(success);

    // Verify it is managed
    let (success, stdout, _) = context.run_staircase(&["list", "--json"]);
    assert!(success);
    assert!(
        stdout.contains("\"is_implicit\": false"),
        "Managed staircase should have is_implicit: false in JSON"
    );

    // Delete managed staircase WITHOUT deleting owned branches
    let (success, stdout, stderr) = context.run_staircase(&["delete", "managed"]);
    assert!(success, "Delete managed failed: {}", stderr);
    assert!(stdout.contains("Deleted staircase 'managed'"));

    // Verify branches are still there
    assert!(
        !context
            .run_git(&["rev-parse", "--verify", "managed-1"])
            .is_empty()
    );
    assert!(
        !context
            .run_git(&["rev-parse", "--verify", "managed-tip"])
            .is_empty()
    );

    // Verify managed records are gone (nothing should have is_implicit: false)
    let (success, stdout, _) = context.run_staircase(&["list", "--json"]);
    assert!(success);
    assert!(
        !stdout.contains("\"is_implicit\": false"),
        "No staircase should be managed anymore"
    );

    // Re-adopt
    let (success, _, _) =
        context.run_staircase(&["adopt", "managed-again", "managed-1", "managed-tip"]);
    assert!(success);

    // Delete managed staircase WITH deleting owned branches
    let (success, _stdout, stderr) =
        context.run_staircase(&["delete", "managed-again", "--delete-owned-branches"]);
    assert!(success, "Delete managed with branches failed: {}", stderr);

    // Verify branches are gone
    let m1_check = Command::new("git")
        .current_dir(context.path())
        .args(["rev-parse", "--verify", "managed-1"])
        .output()
        .unwrap();
    assert!(!m1_check.status.success(), "managed-1 should be deleted");
}

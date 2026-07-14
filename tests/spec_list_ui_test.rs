mod common;
use common::*;

#[test]
fn test_list_implicit_flag_and_output() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // Create implicit staircase with 2 steps
    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // ACT: Run git staircase list --implicit
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--implicit"]);

    // ASSERT: Verify success and output format
    assert!(success, "list --implicit failed: {}", stderr);

    let output = if stdout.starts_with("Configured Staircase workspace:") {
        stdout
            .lines()
            .skip_while(|l| !l.starts_with("feature/"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        stdout
    };
    let output = output.trim();
    assert!(output.starts_with("feature/auth [implicit@"));
    assert!(output.ends_with("] 2 steps clean (implicit)"));
}

#[test]
fn test_list_managed_output() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(dir, &["adopt", "auth", "feature/auth-core"]);
    assert!(success, "adopt failed: {}", stderr);

    // ACT: Run git staircase list --managed
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--managed"]);

    assert!(success, "list --managed failed: {}", stderr);

    let output = stdout.trim();
    assert!(output.starts_with("auth ["));
    assert!(output.ends_with("] 1 step clean"));
}

#[test]
fn test_list_families_flag() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    // Create a shared base
    run_git(dir, &["checkout", "-b", "shared-base"]);
    commit(dir, "shared.txt", "shared", "shared commit");

    // Fork 1: feature/ui
    run_git(dir, &["checkout", "-b", "feature/ui"]);
    commit(dir, "ui.txt", "ui", "ui commit");

    // Fork 2: feature/cli
    run_git(dir, &["checkout", "shared-base"]);
    run_git(dir, &["checkout", "-b", "feature/cli"]);
    commit(dir, "cli.txt", "cli", "cli commit");

    // ACT: Run git staircase list --families
    let (success, stdout, stderr) = run_staircase(dir, &["list", "--families"]);

    // ASSERT: Verify success and output includes families
    assert!(success, "list --families failed: {}", stderr);

    let output = stdout.trim();
    // Example: auth-family 2 paths (implicit)
    // In this case, shared-base has 2 children (feature/ui and feature/cli)
    // The discovery logic should name it after the common prefix or the branches.
    // Based on my run, it seems to be named "feature".
    assert!(
        output.contains("2 paths"),
        "Output should mention 2 paths: {}",
        output
    );
    assert!(
        output.contains("(implicit)"),
        "Output should mention implicit: {}",
        output
    );
}

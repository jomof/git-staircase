
use crate::common::*;
use std::fs;

#[test]
fn test_draft_status_and_classification() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-1"]);
    commit(dir, "base.txt", "base", "base commit");

    // Adopt staircase
    let (success, _, stderr) =
        run_staircase(dir, &["adopt", "auth", "--onto", "main", "feature/auth-1"]);
    assert!(success, "adopt failed: {}", stderr);

    // Clean draft status
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status"]);
    assert!(success, "draft status failed: {}", stderr);
    assert!(stdout.contains("current worktree draft:"));
    assert!(stdout.contains("staged: 0 paths"));
    assert!(stdout.contains("unstaged: 0 paths"));

    // Staged changes
    fs::write(dir.join("staged_file.txt"), "staged content").unwrap();
    run_git(dir, &["add", "staged_file.txt"]);

    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status"]);
    assert!(success, "draft status failed: {}", stderr);
    assert!(stdout.contains("staged: 1 paths"));

    // Unstaged changes
    fs::write(dir.join("base.txt"), "modified base content").unwrap();

    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status"]);
    assert!(success, "draft status failed: {}", stderr);
    assert!(stdout.contains("staged: 1 paths"));
    assert!(stdout.contains("unstaged: 1 paths"));

    // Untracked changes
    fs::write(dir.join("untracked.txt"), "untracked content").unwrap();

    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status"]);
    assert!(success, "draft status failed: {}", stderr);
    assert!(stdout.contains("untracked: 1 paths"));
}

#[test]
fn test_draft_attach_and_detach() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-1"]);
    commit(dir, "base.txt", "base", "base commit");

    let (success, _, stderr) =
        run_staircase(dir, &["adopt", "auth", "--onto", "main", "feature/auth-1"]);
    assert!(success, "adopt failed: {}", stderr);

    // Attach draft to auth step 1 with mode new-step
    let (success, stdout, stderr) =
        run_staircase(dir, &["draft", "attach", "auth", "--mode", "new-step"]);
    assert!(success, "draft attach failed: {}", stderr);
    assert!(stdout.contains("Attached to auth feature/auth-1"));

    // Status shows attachment
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status"]);
    assert!(success, "draft status failed: {}", stderr);
    assert!(stdout.contains("attached to: auth feature/auth-1"));
    assert!(stdout.contains("intent: new-step"));

    // Detach
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "detach"]);
    assert!(success, "draft detach failed: {}", stderr);
    assert!(stdout.contains("Detached draft attachment"));
}

#[test]
fn test_draft_diff() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    fs::write(dir.join("staged.txt"), "staged line\n").unwrap();
    run_git(dir, &["add", "staged.txt"]);

    fs::write(dir.join("unstaged.txt"), "unstaged line\n").unwrap();

    let (success, stdout, stderr) = run_staircase(dir, &["draft", "diff", "--staged"]);
    assert!(success, "draft diff --staged failed: {}", stderr);
    assert!(stdout.contains("staged.txt"));

    let (success, stdout, stderr) = run_staircase(dir, &["draft", "diff", "--untracked"]);
    assert!(success, "draft diff --untracked failed: {}", stderr);
    assert!(stdout.contains("unstaged.txt"));
}

#[test]
fn test_draft_materialize_extend_step() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-1"]);
    commit(dir, "base.txt", "base", "base commit");

    let (success, _, stderr) =
        run_staircase(dir, &["adopt", "auth", "--onto", "main", "feature/auth-1"]);
    assert!(success, "adopt failed: {}", stderr);

    // Staged parser change
    fs::write(dir.join("parser.txt"), "parser code").unwrap();
    run_git(dir, &["add", "parser.txt"]);

    // Unstaged logging change
    fs::write(dir.join("logging.txt"), "logging code").unwrap();

    // Materialize staged changes into auth step 1
    let (success, stdout, stderr) = run_staircase(
        dir,
        &["draft", "materialize", "auth", "-m", "add parser code"],
    );
    assert!(success, "materialize failed: {}", stderr);
    assert!(stdout.contains("Materialized draft as commit"));

    // Verify commit was created with parser.txt
    let (success, stdout, _) = run_staircase(dir, &["commits", "auth"]);
    assert!(success);
    assert!(stdout.contains("add parser code"));

    // Verify logging.txt remains unstaged in worktree
    let (success, stdout, _) = run_staircase(dir, &["draft", "status"]);
    assert!(success);
    assert!(stdout.contains("staged: 0 paths"));
    assert!(stdout.contains("untracked: 1 paths"));
}

#[test]
fn test_draft_materialize_new_step() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-1"]);
    commit(dir, "base.txt", "base", "base commit");

    let (success, _, stderr) =
        run_staircase(dir, &["adopt", "auth", "--onto", "main", "feature/auth-1"]);
    assert!(success, "adopt failed: {}", stderr);

    fs::write(dir.join("new_feature.txt"), "new feature").unwrap();
    run_git(dir, &["add", "new_feature.txt"]);

    let (success, stdout, stderr) = run_staircase(
        dir,
        &[
            "draft",
            "materialize",
            "auth",
            "--new-step",
            "-m",
            "add new step",
        ],
    );
    assert!(success, "materialize --new-step failed: {}", stderr);
    assert!(stdout.contains("Materialized draft"));

    let (success, stdout, _) = run_staircase(dir, &["steps", "auth"]);
    assert!(success);
    assert!(stdout.contains("Step 2:"));
}

#[test]
fn test_draft_snapshot_and_restore() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    fs::write(dir.join("work.txt"), "work in progress").unwrap();
    run_git(dir, &["add", "work.txt"]);

    let (success, stdout, stderr) = run_staircase(dir, &["draft", "snapshot"]);
    assert!(success, "draft snapshot failed: {}", stderr);
    assert!(stdout.contains("Created snapshot"));

    // Extract snapshot ID from output
    let snapshot_id = stdout
        .split_whitespace()
        .find(|w| w.len() == 36 && w.contains('-'))
        .expect("Snapshot UUID not found");

    // Clear worktree
    run_git(dir, &["reset", "--hard", "HEAD"]);
    run_git(dir, &["clean", "-fd"]);

    // Restore snapshot
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "restore", snapshot_id]);
    assert!(success, "draft restore failed: {}", stderr);
    assert!(stdout.contains("Created snapshot") || stdout.contains(snapshot_id));
}

#[test]
fn test_draft_porcelain_and_json() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    fs::write(dir.join("file.txt"), "hello").unwrap();
    run_git(dir, &["add", "file.txt"]);

    // JSON output
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status", "--json"]);
    assert!(success, "draft status --json failed: {}", stderr);
    assert!(stdout.contains("\"basis\":"));
    assert!(stdout.contains("\"staged_paths\":"));

    // Porcelain output
    let (success, stdout, stderr) = run_staircase(dir, &["draft", "status", "--porcelain"]);
    assert!(success, "draft status --porcelain failed: {}", stderr);
    assert!(stdout.contains("draft\t"));
}

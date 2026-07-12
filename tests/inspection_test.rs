mod common;
use common::*;

#[test]
fn test_inspection_commands_on_implicit_staircase() {
    // ARRANGE
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    run_git(dir, &["checkout", "-b", "feature/auth-tests"]);
    let c3 = commit(dir, "file3.txt", "3", "commit 3");

    let name = "feature/auth";

    // ACT & ASSERT: steps
    let (success, stdout, stderr) = run_staircase(dir, &["steps", name]);
    assert!(success, "steps command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains("feature/auth-tests"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains(&c2[..7]));
    assert!(stdout.contains(&c3[..7]));

    // ACT & ASSERT: commits
    let (success, stdout, stderr) = run_staircase(dir, &["commits", name]);
    assert!(success, "commits command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains(&c2[..7]));
    assert!(stdout.contains("feature/auth-tests"));
    assert!(stdout.contains(&c3[..7]));

    // ACT & ASSERT: log
    let (success, stdout, stderr) = run_staircase(dir, &["log", name]);
    assert!(success, "log command failed: {}", stderr);
    assert!(stdout.contains("commit 1"));
    assert!(stdout.contains("commit 2"));
    assert!(stdout.contains("commit 3"));
    assert!(!stdout.contains("initial commit")); // Should only show commits in the staircase

    // ACT & ASSERT: diff
    let (success, stdout, stderr) = run_staircase(dir, &["diff", name]);
    assert!(success, "diff command failed: {}", stderr);
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("file3.txt"));

    // ACT & ASSERT: graph
    let (success, stdout, stderr) = run_staircase(dir, &["graph", name]);
    assert!(success, "graph command failed: {}", stderr);
    assert!(stdout.contains("commit 1"));
    assert!(stdout.contains("commit 2"));
    assert!(stdout.contains("commit 3"));
}

#[test]
fn test_inspection_commands_on_managed_staircase() {
    // ARRANGE
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(
        dir,
        &[
            "adopt",
            "auth",
            "--onto",
            "main",
            "feature/auth-core",
            "feature/auth-ui",
        ],
    );
    assert!(success, "adopt failed: {}", stderr);

    let name = "auth";

    // ACT & ASSERT: steps
    let (success, stdout, stderr) = run_staircase(dir, &["steps", name]);
    assert!(success, "steps command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains(&c2[..7]));

    // ACT & ASSERT: commits
    let (success, stdout, stderr) = run_staircase(dir, &["commits", name]);
    assert!(success, "commits command failed: {}", stderr);
    assert!(stdout.contains("feature/auth-core"));
    assert!(stdout.contains(&c1[..7]));
    assert!(stdout.contains("feature/auth-ui"));
    assert!(stdout.contains(&c2[..7]));
}

#[test]
fn test_status_output_format_alignment() {
    // ARRANGE: Create implicit staircase
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let name = "feature/auth";

    // ACT: Run status
    let (success, stdout, stderr) = run_staircase(dir, &["status", name]);

    // ASSERT
    assert!(success, "status command failed: {}", stderr);

    // Target format per spec:
    // <name> (implicit)
    //   target: <target>
    //   state: <state>
    //   steps: <count>
    //   lineage: none

    let expected = "feature/auth (implicit)\n  target: refs/heads/main\n  state: clean\n  steps: 2\n  lineage: none";
    assert_eq!(stdout, expected, "Status output does not match spec format");
}

#[test]
fn test_status_output_format_alignment_managed() {
    // ARRANGE: Create managed staircase
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    // Adopt it to make it managed
    let (success, _, stderr) = run_staircase(
        dir,
        &[
            "adopt",
            "auth",
            "--onto",
            "main",
            "feature/auth-core",
            "feature/auth-ui",
        ],
    );
    assert!(success, "adopt failed: {}", stderr);

    let name = "auth";

    // ACT: Run status
    let (success, stdout, stderr) = run_staircase(dir, &["status", name]);

    // ASSERT
    assert!(success, "status command failed: {}", stderr);

    assert!(
        stdout.starts_with("auth\n"),
        "Output should start with name 'auth' (got: {})",
        stdout
    );
    assert!(
        stdout.contains("  target: refs/heads/main\n"),
        "Output should contain target"
    );
    assert!(
        stdout.contains("  state: clean\n"),
        "Output should contain state"
    );
    assert!(
        stdout.contains("  steps: 2\n"),
        "Output should contain steps count"
    );

    let lineage_line = stdout
        .lines()
        .find(|l| l.contains("lineage:"))
        .expect("Lineage line missing");
    assert!(
        !lineage_line.contains("none"),
        "Managed staircase should have a lineage ID (got: {})",
        lineage_line
    );
}

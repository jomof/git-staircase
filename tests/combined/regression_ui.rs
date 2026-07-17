use git_staircase::cli::{self, Command};

use crate::common::*;

// --- repro_adoption_ui.rs ---

#[test]
fn test_adoption_ui() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let (_success, stdout, _stderr) =
        run_staircase(dir, &["id", "feature/auth", "--kind", "lineage"]);

    println!("Stdout: {}", stdout);

    // The spec says it should print "adopted implicit staircase 'feature/auth'"
    assert!(stdout.contains("adopted implicit staircase 'feature/auth'"));
}

// --- repro_toolability.rs ---

#[test]
fn test_reorder_json() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let (success, stdout, stderr) = run_staircase(
        dir,
        &["--json", "reorder", "feature/auth", "--steps", "2,1"],
    );

    assert!(success, "reorder --json failed: {}", stderr);
    println!("Stdout: '{}'", stdout);

    // It should output JSON representing the new state or at least a success status
    assert!(!stdout.trim().is_empty(), "JSON output should not be empty");
    serde_json::from_str::<serde_json::Value>(stdout.trim()).expect("Output should be valid JSON");
}

// --- repro_invalid_names.rs ---

#[test]
#[ignore]
fn test_adopt_with_invalid_name() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "valid-branch"]);
    ctx.commit("a.txt", "a", "A");

    // ACT
    let cmd = cli::adopt::Adopt {
        name: "invalid name".to_string(),
        onto: Some("main".to_string()),
        branches: vec!["valid-branch".to_string()],
        build_command: None,
        test_command: None,
        landing_policy: None,
        verify_each_prefix: false,
    };
    let result = cmd.run(&ctx.repo);

    // ASSERT
    assert!(
        result.is_err(),
        "Adopting with a name containing spaces should fail gracefully, but it crashed or succeeded"
    );
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(
        err_msg.contains("bad name") || err_msg.contains("invalid"),
        "Expected graceful validation error, got: {}",
        err_msg
    );
}

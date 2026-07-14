mod common;
use common::TestContext;

#[test]
fn test_error_json_format_on_invalid_command() {
    let ctx = TestContext::new();
    // Use an invalid command
    let (success, _stdout, stderr) = ctx.run_staircase(&["non-existent-command", "--json"]);

    assert!(!success);
    // Even if clap fails to parse the command, we want it to output JSON error if --json was present
    assert!(
        stderr.contains("\"error\":"),
        "Stderr should contain JSON error: {}",
        stderr
    );
    let json: serde_json::Value =
        serde_json::from_str(&stderr).expect("Stderr should be valid JSON");
    assert_eq!(json["error"]["exit_status"], 1);
}

#[test]
fn test_error_porcelain_format_on_invalid_command() {
    let ctx = TestContext::new();
    let (success, _stdout, stderr) = ctx.run_staircase(&["non-existent-command", "--porcelain"]);

    assert!(!success);
    assert!(
        stderr.starts_with("error\t"),
        "Stderr should start with 'error\t': {}",
        stderr
    );
}

#[test]
fn test_error_json_format_on_missing_arg() {
    let ctx = TestContext::new();
    // 'adopt' requires arguments
    let (success, _stdout, stderr) = ctx.run_staircase(&["adopt", "--json"]);

    assert!(!success);
    assert!(
        stderr.contains("\"error\":"),
        "Stderr should contain JSON error: {}",
        stderr
    );
}

#[test]
fn test_error_json_format_using_format_flag() {
    let ctx = TestContext::new();
    let (success, _stdout, stderr) =
        ctx.run_staircase(&["non-existent-command", "--format", "json"]);

    assert!(!success);
    assert!(
        stderr.contains("\"error\":"),
        "Stderr should contain JSON error: {}",
        stderr
    );
}

#[test]
fn test_error_json_format_using_format_eq_flag() {
    let ctx = TestContext::new();
    let (success, _stdout, stderr) = ctx.run_staircase(&["non-existent-command", "--format=json"]);

    assert!(!success);
    assert!(
        stderr.contains("\"error\":"),
        "Stderr should contain JSON error: {}",
        stderr
    );
}

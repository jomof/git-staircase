mod common;
use common::*;

#[test]
fn test_unknown_flag_json_error() {
    let ctx = TestContext::new();
    let (success, stdout, stderr) = run_staircase(ctx.tmp.path(), &["--json", "--unknown-flag"]);

    assert!(!success);
    assert!(stdout.is_empty());

    let json: serde_json::Value =
        serde_json::from_str(&stderr).expect(&format!("stderr should be valid JSON: '{}'", stderr));
    assert_eq!(json["error"]["code"], "validation-failed");
}

#[test]
fn test_unknown_flag_porcelain_error() {
    let ctx = TestContext::new();
    let (success, stdout, stderr) =
        run_staircase(ctx.tmp.path(), &["--porcelain", "--unknown-flag"]);

    assert!(!success);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("error\tvalidation-failed\t"));
}

#[test]
fn test_unknown_flag_format_json_error() {
    let ctx = TestContext::new();
    let (success, stdout, stderr) =
        run_staircase(ctx.tmp.path(), &["--format", "json", "--unknown-flag"]);

    assert!(!success);
    assert!(stdout.is_empty());

    let json: serde_json::Value =
        serde_json::from_str(&stderr).expect(&format!("stderr should be valid JSON: '{}'", stderr));
    assert_eq!(json["error"]["code"], "validation-failed");
}

#[test]
fn test_unknown_flag_format_equals_json_error() {
    let ctx = TestContext::new();
    let (success, stdout, stderr) =
        run_staircase(ctx.tmp.path(), &["--format=json", "--unknown-flag"]);

    assert!(!success);
    assert!(stdout.is_empty());

    let json: serde_json::Value =
        serde_json::from_str(&stderr).expect(&format!("stderr should be valid JSON: '{}'", stderr));
    assert_eq!(json["error"]["code"], "validation-failed");
}

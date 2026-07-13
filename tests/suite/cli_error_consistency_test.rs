use crate::common::*;
use std::process::Command;

#[test]
fn test_error_output_consistency() {
    let ctx = TestContext::new();

    // Command that fails (invalid staircase name)
    let output1 = Command::new(get_bin_path())
        .args(&["show", "nonexistent"])
        .current_dir(&ctx.tmp)
        .output()
        .unwrap();

    let output2 = Command::new(get_bin_path())
        .args(&["status", "nonexistent"])
        .current_dir(&ctx.tmp)
        .output()
        .unwrap();

    let err1 = String::from_utf8_lossy(&output1.stderr);
    let err2 = String::from_utf8_lossy(&output2.stderr);

    assert!(!output1.status.success());
    assert!(!output2.status.success());
    assert_eq!(
        err1, err2,
        "Error messages should be consistent for non-existent staircase"
    );
}

fn get_bin_path() -> String {
    env!("CARGO_BIN_EXE_git-staircase").to_string()
}

mod common;
use common::*;
use std::process::Command;

#[test]
fn test_error_output_consistency() {
    let ctx1 = TestContext::new();
    let output1 = Command::new(get_bin_path())
        .args(&["show", "nonexistent"])
        .current_dir(ctx1.tmp.path())
        .output()
        .unwrap();

    let ctx2 = TestContext::new();
    let output2 = Command::new(get_bin_path())
        .args(&["status", "nonexistent"])
        .current_dir(ctx2.tmp.path())
        .output()
        .unwrap();

    fn filter_bootstrap_msg(s: &str) -> String {
        s.lines()
            .filter(|line| {
                !line.starts_with("Configured Staircase workspace:")
                    && !line.starts_with("  workspace:")
                    && !line.starts_with("  root:")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    let err1 = filter_bootstrap_msg(&String::from_utf8_lossy(&output1.stderr));
    let err2 = filter_bootstrap_msg(&String::from_utf8_lossy(&output2.stderr));

    assert!(!output1.status.success());
    assert!(!output2.status.success());
    assert_eq!(
        err1, err2,
        "Error messages should be consistent for non-existent staircase"
    );
}

fn get_bin_path() -> String {
    let bin_str = env!("CARGO_BIN_EXE_git-staircase");
    let mut bin = std::path::PathBuf::from(bin_str);
    if bin_str.contains("/shadow-") || !bin.exists() {
        let fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join("git-staircase");
        if fallback.exists() {
            bin = fallback;
        }
    }
    bin.to_string_lossy().to_string()
}

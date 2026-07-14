use git_staircase::process::ProcessExecutor;
use std::process::Command;
use std::time::Duration;

#[test]
fn test_executor_large_io() {
    let cmd = Command::new("cat");
    let large_input = "A".repeat(1024 * 1024);
    let executor = ProcessExecutor::new(cmd)
        .stdin(large_input.clone())
        .timeout(Duration::from_secs(5));

    let output = executor.run().expect("Failed to run cat");
    assert_eq!(output.stdout.len(), large_input.len());
    assert!(output.status.success());
}

#[test]
fn test_executor_timeout() {
    let mut cmd = Command::new("sleep");
    cmd.arg("10");
    let executor = ProcessExecutor::new(cmd).timeout(Duration::from_millis(100));

    let result = executor.run();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("timed out") || err.contains("Timeout"),
        "Error message should mention timeout: {}",
        err
    );
}

#[test]
fn test_executor_stderr() {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("echo hello; echo world >&2");
    let executor = ProcessExecutor::new(cmd);

    let output = executor.run().expect("Failed to run sh");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "world");
}

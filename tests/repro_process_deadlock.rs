use git_staircase::process::ProcessExecutor;
use std::process::Command;
use std::time::Duration;

#[test]
fn test_process_executor_deadlock_on_empty_stdin() {
    // ARRANGE: Use 'cat' which waits for stdin to close.
    // We create a ProcessExecutor without providing stdin data.
    let cmd = Command::new("cat");
    let executor = ProcessExecutor::new(cmd).timeout(Duration::from_secs(2)); // Set a short timeout

    // ACT: Run the executor.
    // In the bug case, this will deadlock because the stdin pipe is never closed
    // before the wait() call, and cat waits for the pipe to close.
    let result = executor.run();

    // ASSERT: It should NOT timeout.
    match result {
        Ok(_) => {}
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Process timed out") {
                panic!("BUG REPRODUCED: ProcessExecutor deadlocked and timed out!");
            }
            panic!("Unexpected error: {}", msg);
        }
    }
}

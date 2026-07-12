mod common;
use common::*;

#[test]
fn test_run_with_stdin_deadlock() {
    // ARRANGE
    let ctx = TestContext::new();

    // Create a large amount of input for a command that echoes back
    // We'll use 'cat-file --batch-check' which outputs a line for each input line
    let mut large_input = String::new();
    for _ in 0..100000 {
        large_input.push_str("HEAD\n");
    }

    // ACT & ASSERT
    // This should deadlock and timeout if the pipe buffer fills up
    let result = ctx.repo.run_with_stdin(&["cat-file", "--batch-check"], &large_input);
    assert!(result.is_ok());
}

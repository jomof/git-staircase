mod common;
use common::*;
use git_staircase::StaircaseError;

#[test]
fn test_git_cmd_setup() {
    // ARRANGE
    let ctx = TestContext::new();

    // ACT
    let cmd = ctx.repo.git_cmd();

    // ASSERT
    assert_eq!(cmd.get_program(), "git");
    let output = ctx
        .repo
        .run(&["rev-parse", "--is-inside-work-tree"])
        .unwrap();
    assert_eq!(output.trim(), "true");
}

#[test]
fn test_git_command_failed_captures_info() {
    // ARRANGE
    let ctx = TestContext::new();

    // ACT
    let result = ctx.repo.run(&["rev-parse", "NON_EXISTENT"]);

    // ASSERT
    match result {
        Err(StaircaseError::GitCommandFailed {
            command,
            stdout: _,
            stderr,
        }) => {
            assert!(command.contains("git rev-parse NON_EXISTENT"));
            assert!(stderr.contains("fatal: ambiguous argument 'NON_EXISTENT'"));
        }
        _ => panic!("Expected GitCommandFailed error, got {:?}", result),
    }
}

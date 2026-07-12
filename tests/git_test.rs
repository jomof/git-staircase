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

#[test]
fn test_builder_basic_run() {
    // ARRANGE
    let ctx = TestContext::new();

    // ACT
    let output = ctx.repo.command()
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .run()
        .unwrap();

    // ASSERT
    assert_eq!(output, "true");
}

#[test]
fn test_builder_trim() {
    // ARRANGE
    let ctx = TestContext::new();

    // ACT & ASSERT
    // Default should be trimmed if we return String
    let output = ctx.repo.command()
        .args(&["rev-parse", "--is-inside-work-tree"])
        .run()
        .unwrap();
    assert_eq!(output, "true");

    let output_untrimmed = ctx.repo.command()
        .args(&["rev-parse", "--is-inside-work-tree"])
        .trim(false)
        .run()
        .unwrap();
    assert!(output_untrimmed.ends_with("\n"));
}

#[test]
fn test_builder_stdin() {
    // ARRANGE
    let ctx = TestContext::new();

    // ACT
    let output = ctx.repo.command()
        .args(&["hash-object", "--stdin"])
        .stdin("hello")
        .run()
        .unwrap();
    
    let expected = ctx.repo.run_with_stdin(&["hash-object", "--stdin"], "hello").unwrap();

    // ASSERT
    assert_eq!(output, expected);
}

#[test]
fn test_builder_error_handling() {
    // ARRANGE
    let ctx = TestContext::new();

    // ACT
    let result = ctx.repo.command()
        .args(&["rev-parse", "NON_EXISTENT"])
        .run();
    
    // ASSERT
    assert!(result.is_err());
}

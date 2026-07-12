mod common;
use common::*;
use git_staircase::cli;

#[test]
#[ignore]
fn test_adopt_with_invalid_name() {
    // ARRANGE
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "valid-branch"]);
    ctx.commit("a.txt", "a", "A");
    
    // ACT
    let result = cli::adopt::run(
        &ctx.repo,
        cli::OutputFormat::Human,
        "invalid name".to_string(), // Space in name
        Some("main".to_string()),
        vec!["valid-branch".to_string()],
        None,
        None,
        false
    );

    // ASSERT
    assert!(result.is_err(), "Adopting with a name containing spaces should fail gracefully, but it crashed or succeeded");
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(err_msg.contains("bad name") || err_msg.contains("invalid"), "Expected graceful validation error, got: {}", err_msg);
}

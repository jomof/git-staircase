use git_staircase::cli::formatting::{render_human, render_porcelain};
use git_staircase::presentation::Presentation;
use serde_json::json;

#[test]
fn test_error_presentation_human() {
    let p = Presentation::error(
        "test-error",
        "Something went wrong",
        1,
        json!({"detail": "extra"}),
    );
    let rendered = render_human(&p, 0);
    assert!(rendered.contains("error [test-error]: Something went wrong"));
    assert!(rendered.contains("\"detail\": \"extra\""));
}

#[test]
fn test_error_presentation_porcelain() {
    let p = Presentation::error("test-error", "Something\twent\nwrong", 1, json!(null));
    let rendered = render_porcelain(&p);
    // Should be tab-separated and escaped
    assert_eq!(rendered, "error\ttest-error\tSomething\\twent\\nwrong\n");
}


use crate::common::*;

#[test]
fn test_error_output_consistency() {
    let ctx1 = TestContext::new();
    let (_, _, stderr1) = ctx1.run_staircase(&["show", "nonexistent"]);

    let ctx2 = TestContext::new();
    let (_, _, stderr2) = ctx2.run_staircase(&["status", "nonexistent"]);

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

    let err1 = filter_bootstrap_msg(&stderr1);
    let err2 = filter_bootstrap_msg(&stderr2);

    assert_eq!(
        err1, err2,
        "Error messages should be consistent for non-existent staircase"
    );
}

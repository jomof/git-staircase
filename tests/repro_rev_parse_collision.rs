mod common;
use common::TestContext;

#[test]
fn test_rev_parse_family_panic() {
    let ctx = TestContext::new();
    // ctx.run_git(&["checkout", "-b", "main"]); // Already exists
    ctx.commit("a.txt", "a", "commit a");
    
    // Create two branches from main to form an ambiguous family
    ctx.run_git(&["checkout", "-b", "feature/1", "main"]);
    ctx.commit("1.txt", "1", "commit 1");
    ctx.run_git(&["checkout", "-b", "feature/2", "main"]);
    ctx.commit("2.txt", "2", "commit 2");

    // ACT: Run rev-parse with a structural key that matches the family
    let (_, stdout, _) = ctx.run_staircase(&["discover"]);
    
    // The structural key will look like implicit@...
    let line = stdout.lines().find(|l| l.contains("implicit@")).unwrap();
    let key = line.split_whitespace().find(|w| w.starts_with("implicit@")).unwrap();
    
    // ASSERT: This should not panic
    let (success, _stdout, stderr) = ctx.run_staircase(&["rev-parse", "--structural-key", key]);
    assert!(success, "rev-parse failed: {}", stderr);
}

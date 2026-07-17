
use crate::common::TestContext;

#[test]
fn test_managed_review_flow_gerrit() {
    let ctx = TestContext::new();

    // 1. Create a staircase
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit(
        "file1.txt",
        "content1",
        "step 1\n\nChange-Id: I1234567890123456789012345678901234567890",
    );

    // 2. Adopt as managed
    let (success, _, stderr) = ctx.run_staircase(&["adopt", "my-staircase", "feature"]);
    assert!(success, "adopt failed: {}", stderr);

    // 3. Configure gerrit route so probe works
    ctx.run_git(&["config", "gerrit.host", "gerrit.example.com"]);
    ctx.run_git(&["config", "gerrit.project", "my-project"]);

    // 4. Run review create
    let (success, stdout, stderr) = ctx.run_staircase(&["review", "create", "my-staircase"]);
    assert!(success, "review create failed: {}", stderr);
    assert!(stdout.contains("Gerrit review create: 1 association(s)"));

    // 5. Run review status
    let (success, stdout, stderr) = ctx.run_staircase(&["review", "status", "my-staircase"]);
    assert!(success, "review status failed: {}", stderr);
    assert!(stdout.contains("Gerrit"));
    assert!(stdout.to_lowercase().contains("status: pending"));
}

#[test]
fn test_managed_review_flow_github() {
    let ctx = TestContext::new();

    // 1. Create a staircase
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("file1.txt", "content1", "step 1");

    // 2. Adopt as managed
    let (success, _, stderr) = ctx.run_staircase(&["adopt", "my-staircase", "feature"]);
    assert!(success, "adopt failed: {}", stderr);

    // 3. Configure github route so probe works
    ctx.run_git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/owner/repo.git",
    ]);

    // Note: GitHub review create will fail in blackbox test because it tries real network.
}

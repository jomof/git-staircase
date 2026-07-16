mod common;

use common::*;

#[test]
fn observation_never_adopts() {
    // ARRANGE: Setup a repository with an implicit staircase.
    // An implicit staircase is a branch ahead of the integration anchor (main).
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "feature"]);
    context.commit("feature.txt", "feature content", "feature commit");

    // Verify it is discovered as implicit.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(stdout.contains("(implicit)"));
    assert!(stdout.contains("feature"));

    // ACT: Run exhaustive inspection commands.
    let commands = [
        vec!["list"],
        vec!["show", "feature"],
        vec!["status", "feature"],
        vec!["steps", "feature"],
        vec!["commits", "feature"],
        vec!["discover"],
        vec!["log", "feature"],
        vec!["diff", "feature"],
        vec!["describe", "feature"],
        vec!["id", "feature"],
        vec!["rev-parse", "feature"],
    ];

    for args in commands {
        println!("Running: git-staircase {:?}", args);
        context.run_staircase(&args);

        let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
        for line in refs.lines() {
            assert!(
                !line.starts_with("refs/staircases/"),
                "Found unexpected staircase ref after {:?}: {}",
                args,
                line
            );
            assert!(
                !line.starts_with("refs/staircase-state/"),
                "Found unexpected staircase-state ref after {:?}: {}",
                args,
                line
            );
            assert!(
                !line.starts_with("refs/staircase-archive/"),
                "Found unexpected staircase-archive ref after {:?}: {}",
                args,
                line
            );
        }
    }
}

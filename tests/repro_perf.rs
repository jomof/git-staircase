mod common;
use common::builder::RepoBuilder;
use std::time::Instant;

#[test]
fn test_discovery_performance() {
    let mut builder = RepoBuilder::new()
        .git(&["init", "-b", "main"])
        .git(&["config", "core.hooksPath", "/dev/null"])
        .git(&["config", "user.email", "test@example.com"])
        .git(&["config", "user.name", "Test"])
        .write_file("file", "content")
        .git(&["add", "file"])
        .git(&["commit", "-m", "initial"]);

    // Create 100 branches in a chain.
    for i in 1..=100 {
        builder = builder
            .write_file("file", &format!("content {}", i))
            .git(&["add", "file"])
            .git(&["commit", "-m", &format!("commit {}", i)])
            .git(&["branch", &format!("branch-{}", i)]);
    }

    let (_tmp, repo) = builder.build();

    let start = Instant::now();
    let _discoveries =
        git_staircase::core::discovery::discover(&repo, Some("main"), None, false).unwrap();
    let duration = start.elapsed();

    println!("Discovery of 100 branches took {:?}", duration);
    // If it takes more than 5 seconds for 100 branches, it's definitely a bottleneck
    assert!(
        duration.as_secs() < 5,
        "Discovery took too long: {:?}",
        duration
    );
}

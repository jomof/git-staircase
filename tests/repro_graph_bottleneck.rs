mod common;
use common::builder::RepoBuilder;
use git_staircase::core::graph::build_branch_graph;
use git_staircase::model::BranchInfo;
use std::time::Instant;

#[test]
fn test_build_branch_graph_bottleneck() {
    let mut builder = RepoBuilder::new()
        .git(&["init", "-b", "main"])
        .write_file("file.txt", "content")
        .git(&["add", "."])
        .git(&["commit", "-m", "initial"]);

    let num_branches = 50;
    for i in 1..=num_branches {
        builder = builder
            .write_file(&format!("file{}.txt", i), "content")
            .git(&["add", "."])
            .git(&["commit", "-m", &format!("commit {}", i)])
            .git(&["branch", &format!("b{}", i)]);
    }

    let (_tmp, repo) = builder.build();

    let mut active_branches = Vec::new();
    for i in 1..=num_branches {
        let refname = format!("refs/heads/b{}", i);
        let oid = repo
            .run(&["rev-parse", &refname])
            .unwrap()
            .trim()
            .to_string();
        active_branches.push(BranchInfo {
            refname,
            oid,
            upstream: None,
        });
    }

    // ACT: Measure time to build branch graph
    let start = Instant::now();
    let _ = build_branch_graph(&repo, &active_branches).expect("Failed to build branch graph");
    let duration = start.elapsed();

    println!(
        "Time to build graph for {} branches: {:?}",
        num_branches, duration
    );

    // ASSERT: If it takes more than 2 seconds for 50 branches, it's definitely a bottleneck
    // (In a healthy system, this should be sub-second even with 100 branches)
    assert!(
        duration.as_secs() < 2,
        "build_branch_graph is too slow: {:?}",
        duration
    );
}

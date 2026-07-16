use git_staircase::core::graph::build_branch_graph;
use git_staircase::git::GitRepo;
use git_staircase::model::BranchInfo;
use std::fs;
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_build_branch_graph_bottleneck() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    
    // ARRANGE: Initialize a repo and create many branches in a chain
    Command::new("git").current_dir(dir).args(&["init", "-b", "main"]).output().unwrap();
    fs::write(dir.join("file.txt"), "content").unwrap();
    Command::new("git").current_dir(dir).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(dir).args(&["commit", "-m", "initial"]).output().unwrap();
    
    let mut active_branches = Vec::new();
    let num_branches = 50;
    
    for i in 1..=num_branches {
        fs::write(dir.join(format!("file{}.txt", i)), "content").unwrap();
        Command::new("git").current_dir(dir).args(&["add", "."]).output().unwrap();
        Command::new("git").current_dir(dir).args(&["commit", "-m", &format!("commit {}", i)]).output().unwrap();
        let oid = String::from_utf8(Command::new("git").current_dir(dir).args(&["rev-parse", "HEAD"]).output().unwrap().stdout).unwrap().trim().to_string();
        let refname = format!("refs/heads/b{}", i);
        Command::new("git").current_dir(dir).args(&["branch", &format!("b{}", i)]).output().unwrap();
        active_branches.push(BranchInfo {
            refname,
            oid,
            upstream: None,
        });
    }
    
    let repo = GitRepo::new(dir.to_path_buf());
    
    // ACT: Measure time to build branch graph
    let start = Instant::now();
    let _ = build_branch_graph(&repo, &active_branches).expect("Failed to build branch graph");
    let duration = start.elapsed();
    
    println!("Time to build graph for {} branches: {:?}", num_branches, duration);
    
    // ASSERT: If it takes more than 2 seconds for 50 branches, it's definitely a bottleneck
    // (In a healthy system, this should be sub-second even with 100 branches)
    assert!(duration.as_secs() < 2, "build_branch_graph is too slow: {:?}", duration);
}

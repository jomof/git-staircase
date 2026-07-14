mod common;
use common::*;
use std::time::Instant;

#[test]
fn test_preload_ancestry_performance() {
    let (tmp, repo) = setup_repo();
    let path = tmp.path();

    // Create 100 branches
    let mut oids = Vec::new();
    for i in 0..100 {
        let branch_name = format!("branch_{}", i);
        ctx_run_git(path, &["checkout", "-b", &branch_name, "main"]);
        let oid = ctx_commit(path, &format!("file_{}.txt", i), "content", "msg");
        oids.push(oid);
    }

    let oid_refs: Vec<&str> = oids.iter().map(|s| s.as_str()).collect();

    let start = Instant::now();
    repo.preload_ancestry(&oid_refs).unwrap();
    let duration = start.elapsed();

    println!("Preload ancestry for 100 branches took: {:?}", duration);
    // On a typical machine, this should be very fast (<100ms) if efficient.
    // The current implementation takes significantly longer due to O(N^2) behavior.
}

fn ctx_run_git(dir: &std::path::Path, args: &[&str]) -> String {
    run_git(dir, args)
}

fn ctx_commit(dir: &std::path::Path, file: &str, contents: &str, msg: &str) -> String {
    commit(dir, file, contents, msg)
}

mod common;
use common::*;
use std::time::Instant;

#[test]
fn test_preload_ancestry_performance() {
    let ctx = TestContext::new();
    let path = &ctx.repo.workdir;

    // Create 200 branches to make it more obvious
    let mut oids = Vec::new();
    for i in 0..200 {
        let branch_name = format!("branch_{}", i);
        run_git(path, &["checkout", "-b", &branch_name, "main"]);
        let oid = commit(path, &format!("file_{}.txt", i), "content", "msg");
        oids.push(oid);
    }

    let oid_refs: Vec<&str> = oids.iter().map(|s| s.as_str()).collect();

    let start = Instant::now();
    ctx.repo.preload_ancestry(&oid_refs).unwrap();
    let duration = start.elapsed();

    println!("Preload ancestry for 200 branches took: {:?}", duration);
    assert!(
        duration.as_millis() < 2000,
        "Preload ancestry is too slow: {:?}",
        duration
    );
}

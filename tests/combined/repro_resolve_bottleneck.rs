
use crate::common::*;
use std::time::Instant;

#[test]
fn test_resolve_commit_bottleneck() {
    let (tmp, repo) = setup_repo();
    let path = tmp.path();

    // Create 150 commits (creating thousands takes too long in setup phase)
    let mut oids = Vec::new();
    for i in 0..150 {
        let oid = commit(path, &format!("file_{}.txt", i), "content", "msg");
        oids.push(oid);
    }

    let start = Instant::now();
    for oid in &oids {
        repo.resolve_commit(oid).unwrap();
    }
    let duration = start.elapsed();

    println!("Resolving 150 OIDs took: {:?}", duration);
    // If it takes > 1ms per OID, it's likely hitting git too much.
    // Each resolve_commit should be a single git call (or memoized).
    // But currently it's 3 git calls per OID if not memoized.
    // Even if memoized, the first call is slow.
    assert!(
        duration.as_secs_f32() < 0.5,
        "Resolving 150 OIDs took too long: {:?}",
        duration
    );
}

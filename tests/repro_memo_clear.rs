mod common;
use common::*;
use git_staircase::git::set_git_hook;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_memoization_cleared_on_update_ref() {
    let (tmp, repo) = setup_repo();
    let path = tmp.path().to_path_buf();

    let a = commit(path.as_path(), "a.txt", "a", "a");
    let b = commit(path.as_path(), "b.txt", "b", "b");

    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    set_git_hook(Box::new(move |dir, args, _stdin| {
        if args.contains(&"merge-base".to_string()) && args.contains(&"--is-ancestor".to_string()) {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }
        // Run actual git command to get correct result
        let output = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        Ok(output)
    }));

    // ARRANGE: Memoize ancestry
    assert!(repo.is_ancestor(&a, &b).unwrap());
    let initial_calls = call_count.load(Ordering::SeqCst);
    assert!(initial_calls > 0, "Should have called git merge-base once");

    // ACT: Call again, should be cached
    assert!(repo.is_ancestor(&a, &b).unwrap());
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        initial_calls,
        "Should NOT have called git again"
    );

    // ACT: Update a ref
    repo.update_ref("refs/heads/dummy", &a, None).unwrap();

    // ASSERT: Ancestry should STILL be cached because A and B are immutable OIDs
    assert!(repo.is_ancestor(&a, &b).unwrap());
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        initial_calls,
        "Ancestry should still be cached after update_ref"
    );
}

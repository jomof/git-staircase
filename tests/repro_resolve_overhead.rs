mod common;
use common::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use git_staircase::git::set_git_hook;

#[test]
fn test_resolve_commit_overhead() {
    let (tmp, repo) = setup_repo();
    let path = tmp.path().to_path_buf();
    
    let sha = commit(path.as_path(), "a.txt", "a", "a");

    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    set_git_hook(Box::new(move |dir, args, _stdin| {
        if args.contains(&"rev-parse".to_string()) {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }
        let output = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        Ok(output)
    }));

    // ACT: Resolve a full SHA
    let resolved = repo.resolve_commit(&sha).unwrap();
    assert_eq!(resolved, sha);

    // ASSERT: Should have only called git rev-parse ONCE
    let calls = call_count.load(Ordering::SeqCst);
    assert_eq!(calls, 1, "Should only call rev-parse once for a full SHA, but called {} times", calls);
}

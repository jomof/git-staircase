#[cfg(test)]
mod tests {
    use git_staircase::core::operation::lock::RepositoryLock;
    use git_staircase::git::GitRepo;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_lock_stale_race() {
        let tmp = TempDir::new().unwrap();
        std::process::Command::new("git")
            .arg("init")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        let repo = GitRepo::new(tmp.path().to_path_buf());

        let lock_path = tmp.path().join(".git/staircase/locks/test.lock");
        std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();

        // Create a stale lock file with a non-existent PID
        let stale_lock = serde_json::json!({
            "schema": "git-staircase/operation-lock",
            "version": 1,
            "pid": 999999, // Highly unlikely to exist
            "operation_id": "stale-op",
            "nonce": "stale-nonce",
        });
        fs::write(&lock_path, serde_json::to_string(&stale_lock).unwrap()).unwrap();

        // Now, in a loop, try to acquire the lock while another thread rapidly deletes it.
        // Wait, even simpler: if we can show that RepositoryLock::acquire returns OperationInProgress
        // when the file is missing (but was there a millisecond ago), that's the bug.

        // We can't easily time it perfectly, but we can mock it or just use a very tight loop.

        for _ in 0..100 {
            fs::write(&lock_path, serde_json::to_string(&stale_lock).unwrap()).unwrap();

            // We want RepositoryLock::acquire to hit the "AlreadyExists" branch,
            // then have the file vanish before it reads it.
            let repo_clone = repo.clone();
            let lock_path_clone = lock_path.clone();

            let t = std::thread::spawn(move || {
                let _ = fs::remove_file(lock_path_clone);
            });

            match RepositoryLock::acquire(&repo_clone, "test-op", "test.lock") {
                Ok(lock) => {
                    // Success, good.
                    drop(lock);
                }
                Err(e) => {
                    if format!("{:?}", e).contains("OperationInProgress") {
                        // THIS IS THE BUG! It should have realized the lock was gone and tried again or succeeded.
                        panic!(
                            "Bug reproduced: Got OperationInProgress for a lock that was being deleted: {:?}",
                            e
                        );
                    }
                }
            }
            t.join().unwrap();
        }
    }
}

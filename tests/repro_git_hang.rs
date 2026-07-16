use git_staircase::GitRepo;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[test]
fn test_git_command_hangs_on_unconsumed_stdin() {
    let dir = tempdir().unwrap();
    let repo = GitRepo::new(dir.path().to_path_buf());

    // Create a large amount of data to fill the pipe buffer (usually 64KB)
    let large_data = "A".repeat(1024 * 1024); // 1MB

    let start = Instant::now();

    // Run a command that doesn't read stdin (like 'sleep')
    // We use a timeout to prevent the test itself from hanging indefinitely
    let handle = std::thread::spawn(move || {
        // 'git help' is a good candidate because it's a real git command
        // that doesn't read stdin but might take some time or we can use a no-op script
        // Actually, let's use a script that just sleeps.
        let script_path = dir.path().join("sleep.sh");
        std::fs::write(&script_path, "#!/bin/sh\nsleep 2").unwrap();
        std::process::Command::new("chmod")
            .arg("+x")
            .arg(&script_path)
            .status()
            .unwrap();

        // We use GitCommand via GitRepo.command()
        repo.command()
            .arg("version") // Just a command that doesn't read stdin
            .stdin(large_data)
            .run()
    });

    // Wait for a reasonable time. If it's blocked, it will take at least 2 seconds (the sleep).
    // But if it deadlocks, it might take forever.
    // Wait, 'git version' will finish immediately, but the stdin thread might be blocked.
    // thread::scope will wait for the stdin thread.

    let _res = handle.join();
    let elapsed = start.elapsed();

    println!("Elapsed: {:?}", elapsed);

    // If it took longer than the process execution time, it might be due to the blocked stdin thread.
    // Actually, 'git version' is super fast. If it takes > 1s, it's definitely the bug.
    assert!(
        elapsed < Duration::from_secs(1),
        "Command took {:?}, suspected hang/deadlock on stdin pipe",
        elapsed
    );
}

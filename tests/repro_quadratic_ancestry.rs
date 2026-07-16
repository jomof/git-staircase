use git_staircase::git::GitRepo;
use std::fs;
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

#[test]
#[ignore]
fn test_repro_quadratic_ancestry_bottleneck() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();

    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    };

    run_git(&["init", "-b", "main"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test"]);

    fs::write(repo_path.join("file"), "initial").unwrap();
    run_git(&["add", "file"]);
    run_git(&["commit", "-m", "initial"]);

    // Create 150 branches in a chain.
    for i in 1..=150 {
        fs::write(repo_path.join("file"), format!("content {}", i)).unwrap();
        run_git(&["add", "file"]);
        run_git(&["commit", "-m", &format!("commit {}", i)]);
        run_git(&["branch", &format!("branch-{}", i)]);
    }

    let repo = GitRepo::new(repo_path.to_path_buf());

    let start = Instant::now();
    let _ = git_staircase::core::discovery::discover(&repo, Some("main"), None, false).unwrap();
    let duration = start.elapsed();

    println!("Discovery of 150 branches took {:?}", duration);
    // Discovery should take much less than 500ms for 150 branches.
    assert!(
        duration.as_millis() < 500,
        "Discovery is too slow: {:?}",
        duration
    );
}

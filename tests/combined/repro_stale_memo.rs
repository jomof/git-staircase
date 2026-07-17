use git_staircase::GitRepo;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_stale_memoization_after_checkout() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .status()
            .unwrap();
        assert!(status.success());
    };

    run_git(&["init"]);
    run_git(&["config", "user.email", "you@example.com"]);
    run_git(&["config", "user.name", "Your Name"]);
    fs::write(repo_path.join("file.txt"), "hello").unwrap();
    run_git(&["add", "file.txt"]);
    run_git(&["commit", "-m", "initial"]);
    run_git(&["branch", "other"]);

    let repo = GitRepo::new(repo_path.to_path_buf());

    // ARRANGE: Resolve HEAD symbolic name (cached)
    let head1 = repo.resolve_symbolic_full_name("HEAD").unwrap();

    // ACT: Switch branch using run_interactive (which doesn't clear memoizer)
    repo.run_interactive(&["checkout", "other"]).unwrap();

    // ASSERT: Resolve HEAD again (should return new branch but returns cached old branch)
    let head2 = repo.resolve_symbolic_full_name("HEAD").unwrap();

    assert_ne!(head1, head2, "HEAD should have changed after checkout");
}

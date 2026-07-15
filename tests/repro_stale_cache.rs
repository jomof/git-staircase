use git_staircase::GitRepo;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_stale_cache_after_manual_ref_update() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();

    let run_git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    };

    run_git(&["init", "-b", "main"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test"]);

    fs::write(repo_path.join("file"), "content 1").unwrap();
    run_git(&["add", "file"]);
    run_git(&["commit", "-m", "commit 1"]);
    let oid1 = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let oid1 = String::from_utf8_lossy(&oid1.stdout).trim().to_string();

    fs::write(repo_path.join("file"), "content 2").unwrap();
    run_git(&["add", "file"]);
    run_git(&["commit", "-m", "commit 2"]);
    let oid2 = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let oid2 = String::from_utf8_lossy(&oid2.stdout).trim().to_string();

    run_git(&["branch", "test-branch", &oid2]);

    let repo = GitRepo::new(repo_path.to_path_buf());

    // Resolve branch - this caches it
    let resolved_oid = repo.resolve_commit("test-branch").unwrap();
    assert_eq!(resolved_oid, oid2);

    // Manually move the branch outside of GitRepo's tracked methods
    // Use repo.run to simulate a git command that updates refs
    repo.run(&["branch", "-f", "test-branch", &oid1]).unwrap();

    // Resolve again
    let resolved_oid_again = repo.resolve_commit("test-branch").unwrap();

    // ASSERT: the cache should be cleared, but it isn't, so it returns oid2 instead of oid1
    assert_eq!(
        resolved_oid_again, oid1,
        "Cache was stale! Expected {} but got {}",
        oid1, resolved_oid_again
    );
}

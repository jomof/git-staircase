use git_staircase::GitRepo;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_ref_precedence_tags_vs_heads() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    let run_git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Git command failed: git {:?}\nStderr: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    run_git(&["init"]);
    run_git(&["config", "user.email", "you@example.com"]);
    run_git(&["config", "user.name", "Your Name"]);

    fs::write(repo_path.join("file.txt"), "first").unwrap();
    run_git(&["add", "file.txt"]);
    run_git(&["commit", "-m", "first"]);
    let commit1 = run_git(&["rev-parse", "HEAD"]);

    // Create a tag 'v1' pointing to commit1
    run_git(&["tag", "v1", &commit1]);

    fs::write(repo_path.join("file.txt"), "second").unwrap();
    run_git(&["add", "file.txt"]);
    run_git(&["commit", "-m", "second"]);
    let commit2 = run_git(&["rev-parse", "HEAD"]);

    // Create a branch 'v1' pointing to commit2
    run_git(&["branch", "v1", &commit2]);

    let repo = GitRepo::new(repo_path.to_path_buf());

    // In Git, 'v1' should resolve to the TAG 'v1' (commit1)
    let git_resolved = run_git(&["rev-parse", "v1"]);
    assert_eq!(
        git_resolved, commit1,
        "Git should resolve 'v1' to the tag (commit1)"
    );

    // ACT: Resolve 'v1' using our repo.resolve_commit
    let our_resolved = repo.resolve_commit("v1").unwrap();

    // ASSERT: It should be commit1, but it incorrectly resolves to commit2 (the branch).
    assert_eq!(
        our_resolved, commit1,
        "resolve_commit should favor tags over heads for 'v1'"
    );
}

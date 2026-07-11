use git_staircase::core::discover;
use git_staircase::git::GitRepo;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn setup_repo() -> (TempDir, GitRepo) {
    let dir = TempDir::new().unwrap();
    let repo_path = dir.path().to_path_buf();

    let run = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(&repo_path)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    };

    run(&["init"]);
    run(&["config", "user.email", "you@example.com"]);
    run(&["config", "user.name", "Your Name"]);
    run(&["commit", "--allow-empty", "-m", "initial commit"]);

    // Create a commit on main
    fs::write(repo_path.join("file"), "content").unwrap();
    run(&["add", "file"]);
    run(&["commit", "-m", "base commit"]);

    // Create two branches at the same OID, ahead of main
    run(&["checkout", "-b", "branch-a"]);
    fs::write(repo_path.join("file"), "modified").unwrap();
    run(&["add", "file"]);
    run(&["commit", "-m", "work"]);

    run(&["branch", "branch-b", "branch-a"]);

    (dir, GitRepo::new(repo_path))
}

#[test]
fn test_discovery_with_duplicate_oids() {
    let (_dir, repo) = setup_repo();
    let onto = if repo.resolve_ref("master").is_ok() {
        "master"
    } else {
        "main"
    };
    let discoveries = discover(&repo, Some(onto)).unwrap();

    let found = discoveries.iter().any(|d| match d {
        git_staircase::model::Discovery::Linear(m) => m
            .steps
            .iter()
            .any(|s| s.name == "branch-a" || s.name == "branch-b"),
        git_staircase::model::Discovery::Ambiguous(f) => {
            f.steps.contains_key("branch-a") || f.steps.contains_key("branch-b")
        }
    });

    assert!(
        found,
        "Should have discovered branch-a and branch-b even if they point to the same OID"
    );
}

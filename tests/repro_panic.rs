use git_staircase::core::{ResolvedStaircase, discover};
use git_staircase::git::GitRepo;
use git_staircase::model::Discovery;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_implicit_family_metadata_panic() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();
    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .status()
            .unwrap();
        assert!(status.success());
    };
    run_git(&["init", "-b", "main"]);
    fs::write(repo_path.join("file"), "0").unwrap();
    run_git(&["add", "file"]);
    run_git(&["commit", "-m", "initial"]);
    run_git(&["checkout", "-b", "staircase-root"]);
    fs::write(repo_path.join("file"), "r").unwrap();
    run_git(&["commit", "-am", "root commit"]);
    run_git(&["checkout", "staircase-root"]);
    run_git(&["checkout", "-b", "branch-a"]);
    fs::write(repo_path.join("file"), "a").unwrap();
    run_git(&["commit", "-am", "commit a"]);
    run_git(&["checkout", "staircase-root"]);
    run_git(&["checkout", "-b", "branch-b"]);
    fs::write(repo_path.join("file"), "b").unwrap();
    run_git(&["commit", "-am", "commit b"]);

    let repo = GitRepo::new(repo_path.to_path_buf());
    let discoveries = discover(&repo, Some("main")).unwrap();
    let family = discoveries
        .iter()
        .find_map(|d| match d {
            Discovery::Ambiguous(f) => Some(f.clone()),
            _ => None,
        })
        .expect("Should have found an ambiguous family");

    let resolved = ResolvedStaircase::ImplicitFamily(family);
    let result = std::panic::catch_unwind(|| {
        let _ = resolved.metadata();
    });
    assert!(
        result.is_err(),
        "Expected .metadata() to panic for ImplicitFamily"
    );
}

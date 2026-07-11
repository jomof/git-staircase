use std::fs;
use std::path::Path;
use std::process::Command;

fn run_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_verify_leaves_detached_head() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_dir = tmp.path();

    run_git(repo_dir, &["init", "-b", "main"]);
    fs::write(repo_dir.join("a.txt"), "a").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "initial"]);

    run_git(repo_dir, &["checkout", "-b", "s1"]);
    fs::write(repo_dir.join("s1.txt"), "s1").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "s1 commit"]);
    let c1 = run_git(repo_dir, &["rev-parse", "HEAD"]);

    run_git(repo_dir, &["checkout", "-b", "s2"]);
    fs::write(repo_dir.join("s2.txt"), "s2").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "s2 commit"]);
    let c2 = run_git(repo_dir, &["rev-parse", "HEAD"]);

    let repo = git_staircase::GitRepo::new(repo_dir.to_path_buf());
    let sc = git_staircase::StaircaseMetadata {
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            git_staircase::Step {
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            git_staircase::Step {
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: Some(git_staircase::VerificationPolicy {
            build_command: Some("true".to_string()),
            test_command: None,
            verify_each_prefix: true,
        }),
    };
    git_staircase::core::adopt(&repo, &sc).unwrap();

    run_git(repo_dir, &["checkout", "main"]);

    // Force a checkout failure on s2 by creating an untracked file that conflicts
    fs::write(repo_dir.join("s2.txt"), "blocker").unwrap();

    let res = git_staircase::core::verify(None, &repo, "test", None, None, None, None);
    assert!(res.is_err());

    let current = run_git(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(
        current, "main",
        "Branch was NOT restored after checkout failure at step 2!"
    );
}

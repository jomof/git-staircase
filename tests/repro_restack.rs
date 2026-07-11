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
    if !output.status.success() {
        panic!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_restack_propagation() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_dir = tmp.path();

    // 1. Initialize repo
    run_git(repo_dir, &["init", "-b", "main"]);
    fs::write(repo_dir.join("a.txt"), "a").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "initial"]);

    // 2. Create a chain of 3 branches: s1 -> s2 -> s3
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

    run_git(repo_dir, &["checkout", "-b", "s3"]);
    fs::write(repo_dir.join("s3.txt"), "s3").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "s3 commit"]);
    let c3 = run_git(repo_dir, &["rev-parse", "HEAD"]);

    // 3. Adopt as a staircase
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
            git_staircase::Step {
                name: "s3".to_string(),
                cut: c3.clone(),
                branch: Some("s3".to_string()),
            },
        ],
        verification_policy: None,
    };
    git_staircase::core::adopt(&repo, &sc).unwrap();

    // 4. Modify s1 by rebasing it onto a new commit on main
    run_git(repo_dir, &["checkout", "main"]);
    fs::write(repo_dir.join("main2.txt"), "main2").unwrap();
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "main update"]);

    run_git(repo_dir, &["checkout", "s1"]);
    run_git(repo_dir, &["rebase", "main"]);
    let s1_new = run_git(repo_dir, &["rev-parse", "HEAD"]);

    let rs = git_staircase::core::resolve_staircase(&repo, "test", None)
        .unwrap()
        .unwrap();

    // 5. Run restack
    git_staircase::core::restack(&repo, &rs).unwrap();

    // 6. Verify results
    let s1_final = run_git(repo_dir, &["rev-parse", "s1"]);
    let s2_final = run_git(repo_dir, &["rev-parse", "s2"]);
    let s3_final = run_git(repo_dir, &["rev-parse", "s3"]);

    assert_eq!(s1_final, s1_new);

    // s2 should have been rebased onto s1_new
    assert!(
        Command::new("git")
            .current_dir(repo_dir)
            .args(["merge-base", "--is-ancestor", &s1_final, &s2_final])
            .status()
            .unwrap()
            .success()
    );

    // s3 SHOULD have been rebased onto s2_new
    assert!(
        Command::new("git")
            .current_dir(repo_dir)
            .args(["merge-base", "--is-ancestor", &s2_final, &s3_final])
            .status()
            .unwrap()
            .success(),
        "s3 was not rebased onto s2"
    );
}

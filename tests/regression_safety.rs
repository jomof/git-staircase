use git_staircase::core::*;
use git_staircase::model::*;
use git_staircase::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

mod common;
use common::*;

// --- repro_checkout_guard.rs ---

#[test]
fn test_checkout_guard_detached_head() -> anyhow::Result<()> {
    // ARRANGE: Setup a git repo with two commits
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    let oid1 = run_git(repo_path, &["rev-parse", "HEAD"]);

    let oid2 = commit(repo_path, "file2", "content2", "commit 2");

    // Go to detached HEAD at commit 1
    run_git(repo_path, &["checkout", &oid1]);

    // Setup an implicit staircase to verify
    run_git(repo_path, &["branch", "-f", "main", &oid1]);
    run_git(repo_path, &["branch", "step1", &oid2]);

    let rs = resolve_staircase(&repo, "step1", Some("main"))?.expect("Staircase step1 not found");

    // ACT: Run verify (this will checkout oid2 and then try to restore)
    let _ = verify(&repo, &rs, None, None, None, None)?;

    // ASSERT: Check if we are back at oid1
    let current_oid = run_git(repo_path, &["rev-parse", "HEAD"]);

    assert_eq!(
        current_oid, oid1,
        "Should have restored to the original OID"
    );

    Ok(())
}

// --- repro_data_loss.rs ---

fn run_git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_reorder_data_loss_on_dirty_workdir() {
    // ARRANGE
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);

    fs::write(dir.join("file.txt"), "base").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "initial"]);

    fs::write(dir.join("file.txt"), "step1").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "step1"]);
    let s1_oid = run_git(dir, &["rev-parse", "HEAD"]);

    // Dirty change to a TRACKED file
    fs::write(dir.join("file.txt"), "dirty precious data").unwrap();

    let repo = GitRepo::new(dir.to_path_buf());
    let metadata = StaircaseMetadata {
        landing_policy: None,
        id: "test".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "s1".to_string(),
            cut: s1_oid.clone(),
            branch: None,
        }],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };
    let rs = ResolvedStaircase::Implicit(metadata);

    // ACT
    // Run reorder. It will trigger a rebase failure or just the finalize() call.
    let _ =
        core::manipulation::reorder(&repo, &rs, &[0], core::ReorderOptions { no_restack: false });

    // ASSERT
    let content = fs::read_to_string(dir.join("file.txt")).unwrap();
    assert_eq!(
        content, "dirty precious data",
        "Data loss! file.txt was reverted by checkout -f"
    );
}

// --- repro_panic.rs ---

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
    let discoveries = discover(&repo, Some("main"), None, true).unwrap();
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

// --- repro_deadlock.rs ---

#[test]
#[ignore]
fn test_run_with_stdin_deadlock() {
    // ARRANGE
    let ctx = TestContext::new();

    // Create a large amount of input for a command that echoes back
    // We'll use 'cat-file --batch-check' which outputs a line for each input line
    let mut large_input = String::new();
    for _ in 0..100000 {
        large_input.push_str("HEAD\n");
    }

    // ACT & ASSERT
    // This should deadlock and timeout if the pipe buffer fills up
    let result = ctx
        .repo
        .run_with_stdin(&["cat-file", "--batch-check"], &large_input);
    assert!(result.is_ok());
}

// --- repro_verify_2.rs ---

#[test]
fn test_verify_leaves_detached_head() {
    let (_tmp, repo) = setup_repo();
    let repo_dir = &repo.workdir;

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

    let sc = StaircaseMetadata {
        landing_policy: None,
        id: "test-sc".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: String::new(),
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                id: String::new(),
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: Some(VerificationPolicy {
            build_command: Some("true".to_string()),
            test_command: None,
            verify_each_prefix: true,
        }),

        primary_branch_layout: None,
        branch_layout_base: None,
    };
    git_staircase::core::adopt(&repo, &sc).unwrap();

    run_git(repo_dir, &["checkout", "main"]);

    // Force a checkout failure on s2 by creating an untracked file that conflicts
    fs::write(repo_dir.join("s2.txt"), "blocker").unwrap();

    let res = git_staircase::core::verify(
        &repo,
        &ResolvedStaircase::Managed(sc),
        None,
        None,
        None,
        None,
    );
    assert!(res.is_err());

    let current = run_git(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(
        current, "main",
        "Branch was NOT restored after checkout failure at step 2!"
    );
}

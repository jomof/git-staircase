use git_staircase::core;
use git_staircase::{GitRepo, IdentityKind};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

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
    assert!(
        output.status.success(),
        "git {:?} failed. Stderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
    let path = dir.join(file);
    fs::write(path, contents).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

fn setup_repo() -> (TempDir, GitRepo) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    (tmp, GitRepo::new(path))
}

#[test]
fn test_implicit_staircase_operations() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create linear chain: main -> feature/auth-core -> feature/auth-ui
    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let name = "feature/auth";

    // 1. resolve_staircase
    let rs = core::resolve_staircase(&repo, name)
        .unwrap()
        .expect("Should find implicit staircase");
    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    assert_eq!(metadata.name, name);
    assert_eq!(metadata.steps.len(), 2);

    // 2. get_status_metadata
    let status = core::get_status_metadata(&repo, metadata.clone()).unwrap();
    assert!(status.is_clean);
    assert_eq!(status.steps[0].actual_oid, Some(c1));
    assert_eq!(status.steps[1].actual_oid, Some(c2));

    // 3. compute_identity
    let id_nominal = core::compute_identity(&repo, metadata, IdentityKind::Nominal).unwrap();
    assert_eq!(id_nominal, name);

    let id_revision = core::compute_identity(&repo, metadata, IdentityKind::Revision).unwrap();
    assert!(!id_revision.is_empty());

    // 4. verify (just check it doesn't crash, since we don't have policy)
    let results = core::verify(&repo, name, None, None, Some(false), Some(true)).unwrap();
    assert_eq!(results.len(), 2); // No steps to verify if no policy and no aggregate?
    // Wait, verify without policy might just return empty or error.
}

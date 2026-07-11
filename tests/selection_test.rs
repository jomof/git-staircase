use git_staircase::GitRepo;
use git_staircase::core;
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

#[test]
fn test_resolve_explicit_staircase() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    run_git(&path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    let c2 = commit(&path, "feat2.txt", "2", "feat 2");

    let rs = core::resolve_explicit_staircase(
        &repo,
        &["feat-1".to_string(), "feat-2".to_string()],
        Some("main"),
    )
    .unwrap();

    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    assert_eq!(metadata.steps.len(), 2);
    assert_eq!(metadata.steps[0].name, "feat-1");
    assert_eq!(metadata.steps[0].cut, c1);
    assert_eq!(metadata.steps[1].name, "feat-2");
    assert_eq!(metadata.steps[1].cut, c2);
    assert_eq!(metadata.target, "main");
}

#[test]
fn test_resolve_by_oid_sub_staircase() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    run_git(&path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    let _c2 = commit(&path, "feat2.txt", "2", "feat 2");

    // Resolve by c1 OID (which is the cut of feat-1)
    let rs = core::resolve_staircase(&repo, &c1, Some("main"))
        .unwrap()
        .expect("Should resolve by OID");

    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    assert_eq!(metadata.steps.len(), 1);
    assert_eq!(metadata.steps[0].name, "feat-1");
    assert_eq!(metadata.steps[0].cut, c1);
}

#[test]
fn test_resolve_from_ambiguous_family() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    // Create a fork (ambiguous family):
    // main -> step1 -> step2a
    //              -> step2b
    run_git(&path, &["checkout", "-b", "step1"]);
    let c1 = commit(&path, "file1.txt", "1", "commit 1");

    run_git(&path, &["checkout", "-b", "step2a"]);
    let c2a = commit(&path, "file2a.txt", "2a", "commit 2a");

    run_git(&path, &["checkout", "step1"]);
    run_git(&path, &["checkout", "-b", "step2b"]);
    let _c2b = commit(&path, "file2b.txt", "2b", "commit 2b");

    // Resolving by name "step" should be ambiguous (multiple implicit staircases if we try to linearize?
    // Actually, discover returns Ambiguous(Family) because of the fork.
    // If we try to resolve by name "step2a" or by OID c2a, it should extract the path to it.

    // Resolve by OID c2a
    let rs = core::resolve_staircase(&repo, &c2a, Some("main"))
        .unwrap()
        .expect("Should resolve from family by OID");

    assert!(!rs.is_managed());
    let metadata = rs.metadata();
    // Path should be: step1 -> step2a
    assert_eq!(metadata.steps.len(), 2);
    assert_eq!(metadata.steps[0].name, "step1");
    assert_eq!(metadata.steps[0].cut, c1);
    assert_eq!(metadata.steps[1].name, "step2a");
    assert_eq!(metadata.steps[1].cut, c2a);
}

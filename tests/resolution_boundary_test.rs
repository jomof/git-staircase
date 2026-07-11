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
fn test_resolve_staircase_inferred_develop() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "develop"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    run_git(&path, &["checkout", "-b", "feat-1"]);
    commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    commit(&path, "feat2.txt", "2", "feat 2");

    let rs = core::resolve_staircase(&repo, "feat", None)
        .unwrap()
        .expect("Should find implicit staircase by inferring develop");
    assert_eq!(rs.metadata().name, "feat");
    assert_eq!(rs.metadata().target, "develop");
}

#[test]
fn test_resolve_staircase_with_explicit_onto() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "develop"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    run_git(&path, &["checkout", "-b", "feat-1"]);
    commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    commit(&path, "feat2.txt", "2", "feat 2");

    let rs = core::resolve_staircase(&repo, "feat", Some("develop"))
        .unwrap()
        .expect("Should find staircase relative to develop explicitly");
    assert_eq!(rs.metadata().steps.len(), 2);
    assert_eq!(rs.metadata().target, "develop");
}

#[test]
fn test_resolve_staircase_inference_upstream() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");

    run_git(&path, &["checkout", "-b", "base"]);
    commit(&path, "base.txt", "base", "base commit");

    run_git(&path, &["checkout", "-b", "feat-1"]);
    commit(&path, "feat1.txt", "1", "feat 1");

    run_git(&path, &["checkout", "-b", "feat-2"]);
    commit(&path, "feat2.txt", "2", "feat 2");

    run_git(&path, &["branch", "--set-upstream-to=base"]);

    let repo = GitRepo::new(path.clone());

    let rs = core::resolve_staircase(&repo, "feat", None)
        .unwrap()
        .expect("Should infer base as boundary via upstream");
    assert_eq!(rs.metadata().target, "refs/heads/base");
}

use git_staircase::GitRepo;
use git_staircase::core;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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

fn commit(dir: &std::path::Path, file: &str, content: &str, msg: &str) -> String {
    fs::write(dir.join(file), content).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", msg]);
    run_git(dir, &["rev-parse", "HEAD"])
}

#[test]
#[ignore]
fn repro_move_panic() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run_git(dir, &["init", "-b", "main"]);
    commit(dir, "init.txt", "initial", "initial commit");

    // Create 3 steps
    run_git(dir, &["checkout", "-b", "step1"]);
    let c1 = commit(dir, "1.txt", "1", "c1");

    run_git(dir, &["checkout", "-b", "step2"]);
    // Step 2 is EMPTY (at the same commit as step 1)
    let _c2 = c1.clone();

    run_git(dir, &["checkout", "-b", "step3"]);
    let _c3 = commit(dir, "3.txt", "3", "c3");

    let repo = GitRepo::new(dir.to_path_buf());

    // Discover and adopt
    let discoveries = core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::Discovery::Linear(s) = discoveries[0].clone() else {
        panic!()
    };
    let managed = core::adopt(&repo, &s).unwrap();

    let rs = git_staircase::ResolvedSelector {
        staircase: core::resolve_by_id(&repo, &managed.id).unwrap(),
        step_index: None,
    };

    println!("Attempting to move commit from step 1 to step 3...");
    // This should trigger the panic because step 2 is empty.
    core::move_commits(&repo, &rs, 0, 2, &[c1]).unwrap();
    println!("Success!");
}

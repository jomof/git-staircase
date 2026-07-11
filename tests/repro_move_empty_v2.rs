use git_staircase::core::{ResolvedStaircase, adopt, manipulation};
use git_staircase::git::GitRepo;
use git_staircase::model::{StaircaseMetadata, Step};
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
fn test_move_commit_creates_empty_step_violating_invariant() {
    // ARRANGE
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();
    run_git(repo_path, &["init", "-b", "main"]);
    let target = commit(repo_path, "init.txt", "init", "initial");
    let c1 = commit(repo_path, "f1.txt", "1", "c1");
    let c2 = commit(repo_path, "f2.txt", "2", "c2");

    let repo = GitRepo::new(repo_path.to_path_buf());
    let meta = StaircaseMetadata {
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: target,
        steps: vec![
            Step {
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: Some("s1".to_string()),
            },
            Step {
                name: "s2".to_string(),
                cut: c2.clone(),
                branch: Some("s2".to_string()),
            },
        ],
        verification_policy: None,
    };
    let rs = ResolvedStaircase::Managed(meta.clone());
    repo.write_metadata(&meta).unwrap();

    // ACT: Move the only commit of s2 into s1
    manipulation::move_commits(&repo, &rs, 1, 0, &[c2.clone()]).unwrap();

    // ASSERT: The resulting staircase is now considered invalid by the core logic
    let updated_meta = repo.read_metadata("test-id").unwrap();
    let result = adopt(&repo, &updated_meta);
    assert!(
        result.is_err(),
        "Staircase with empty step should be rejected by validation logic"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("every step must be non-empty")
    );
}

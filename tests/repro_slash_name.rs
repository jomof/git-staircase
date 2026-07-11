use git_staircase::core::persistence;
use git_staircase::git::GitRepo;
use git_staircase::model::{StaircaseMetadata, Step};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_slash_name_discovery() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().to_path_buf();

    Command::new("git")
        .current_dir(&repo_path)
        .args(["init", "-b", "main"])
        .status()
        .unwrap();
    fs::write(repo_path.join("init.txt"), "init").unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["add", "."])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "-m", "initial"])
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t")
        .status()
        .unwrap();
    let initial_oid = Command::new("git")
        .current_dir(&repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap()
        .stdout;
    let initial_oid = String::from_utf8(initial_oid).unwrap().trim().to_string();

    let repo = GitRepo::new(repo_path);
    let metadata = StaircaseMetadata {
        id: "uuid".to_string(),
        name: "feature/foo".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            name: "s1".to_string(),
            cut: initial_oid,
            branch: None,
        }],
        verification_policy: None,
    };

    persistence::write_metadata(&repo, &metadata).unwrap();

    let list = persistence::list_staircases(&repo).unwrap();
    assert!(
        list.iter().any(|s| s.name == "feature/foo"),
        "Staircase with slash in name should be listed"
    );
}

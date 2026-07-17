use git_staircase::git::GitRepo;
use git_staircase::memoization::Memoizer;
use std::process::Command;
use tempfile::TempDir;

fn setup_repo(path: std::path::PathBuf) -> GitRepo {
    Command::new("git")
        .current_dir(&path)
        .args(&["init", "-b", "main"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&path)
        .args(&["config", "user.name", "Test"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&path)
        .args(&["config", "user.email", "test@example.com"])
        .output()
        .unwrap();

    std::fs::write(path.join("file.txt"), "content").unwrap();
    Command::new("git")
        .current_dir(&path)
        .args(&["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(&path)
        .args(&["commit", "-m", "initial"])
        .output()
        .unwrap();

    GitRepo::new(path)
}

#[test]
fn test_memoizer_cross_repo_pollution() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();

    let shared_memoizer = Memoizer::new();

    let mut repo1 = setup_repo(tmp1.path().to_path_buf());
    repo1.memoizer = shared_memoizer
        .clone()
        .with_namespace(tmp1.path().to_string_lossy().to_string());

    let mut repo2 = setup_repo(tmp2.path().to_path_buf());
    // Create a different commit in repo2
    std::fs::write(tmp2.path().join("file2.txt"), "different").unwrap();
    Command::new("git")
        .current_dir(tmp2.path())
        .args(&["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(tmp2.path())
        .args(&["commit", "-m", "second"])
        .output()
        .unwrap();
    repo2.memoizer = shared_memoizer.with_namespace(tmp2.path().to_string_lossy().to_string());

    let head1 = repo1.resolve_commit("main").unwrap();
    let head2_actual = repo2.command().args(&["rev-parse", "main"]).run().unwrap();

    assert_ne!(head1, head2_actual, "Repos should have different main OIDs");

    // This should resolve main in repo2, but it might get the cached value from repo1!
    let head2_resolved = repo2.resolve_commit("main").unwrap();

    // ARRANGE/ACT/ASSERT
    assert_eq!(
        head2_resolved, head2_actual,
        "Repo 2 main resolution polluted by Repo 1 cache! Got {}, expected {}",
        head2_resolved, head2_actual
    );
}

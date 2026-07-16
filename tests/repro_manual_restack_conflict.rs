use git_staircase::core::restack::{RestackStrategy, Restacker};
use git_staircase::git::GitRepo;
use git_staircase::model::Step;
use std::process::Command;
use tempfile::TempDir;

fn setup_repo(path: std::path::PathBuf) -> GitRepo {
    Command::new("git").current_dir(&path).args(&["init", "-b", "main"]).output().unwrap();
    Command::new("git").current_dir(&path).args(&["config", "user.name", "Test"]).output().unwrap();
    Command::new("git").current_dir(&path).args(&["config", "user.email", "test@example.com"]).output().unwrap();
    
    std::fs::write(path.join("file.txt"), "initial").unwrap();
    Command::new("git").current_dir(&path).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(&path).args(&["commit", "-m", "initial"]).output().unwrap();
    
    GitRepo::new(path)
}

#[test]
fn test_manual_restack_ignores_conflicts() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path().to_path_buf());
    let initial_oid = repo.resolve_commit("main").unwrap();
    
    // Create base commit C1 changing file.txt
    std::fs::write(tmp.path().join("file.txt"), "c1").unwrap();
    Command::new("git").current_dir(tmp.path()).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(tmp.path()).args(&["commit", "-m", "c1"]).output().unwrap();
    let c1 = repo.resolve_commit("HEAD").unwrap();
    
    // Create diverging commit C2 from initial, also changing file.txt (CONFLICT!)
    Command::new("git").current_dir(tmp.path()).args(&["checkout", "-b", "feature", &initial_oid]).output().unwrap();
    std::fs::write(tmp.path().join("file.txt"), "c2").unwrap();
    Command::new("git").current_dir(tmp.path()).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(tmp.path()).args(&["commit", "-m", "c2"]).output().unwrap();
    let c2 = repo.resolve_commit("HEAD").unwrap();
    
    let restacker = Restacker::prepare(&repo, &[]).unwrap();
    let step = Step { id: "S1".into(), name: "S1".into(), cut: c2.clone(), branch: None };
    
    // Attempt to restack C2 on top of C1 using Manual strategy
    // This should fail because of conflicts, but it currently SUCCEEDS and creates a commit with markers.
    let res = restacker.restack_step(&step, &c2, &initial_oid, &c1, RestackStrategy::Manual);
    
    // ARRANGE/ACT/ASSERT
    assert!(res.is_err(), "Restack should fail on conflicts, but it succeeded: {:?}", res.unwrap());
}

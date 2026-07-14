use git_staircase::git::GitRepo;
use git_staircase::core::inference::infer_onto;
use std::process::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_infer_onto_misses_rebase_in_worktree() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    
    let main_dir = dir.join("main");
    fs::create_dir(&main_dir).unwrap();
    Command::new("git").current_dir(&main_dir).args(["init", "-b", "main"]).output().unwrap();
    Command::new("git").current_dir(&main_dir).args(["config", "user.email", "test@example.com"]).output().unwrap();
    Command::new("git").current_dir(&main_dir).args(["config", "user.name", "test"]).output().unwrap();
    
    fs::write(main_dir.join("file.txt"), "1").unwrap();
    Command::new("git").current_dir(&main_dir).args(["add", "."]).output().unwrap();
    Command::new("git").current_dir(&main_dir).args(["commit", "-m", "c1"]).output().unwrap();
    
    fs::write(main_dir.join("file.txt"), "2").unwrap();
    Command::new("git").current_dir(&main_dir).args(["commit", "-a", "-m", "c2"]).output().unwrap();
    
    // Create a worktree
    let wt_dir = dir.join("wt");
    Command::new("git").current_dir(&main_dir).args(["worktree", "add", wt_dir.to_str().unwrap(), "HEAD^"]).output().unwrap();
    
    // In the worktree, start a rebase that will conflict
    fs::write(wt_dir.join("file.txt"), "conflict").unwrap();
    Command::new("git").current_dir(&wt_dir).args(["commit", "-a", "-m", "conflict-ready"]).output().unwrap();
    
    // This rebase should pause due to conflict
    let _ = Command::new("git").current_dir(&wt_dir).args(["rebase", "main"]).output();
    
    let repo = GitRepo::new(wt_dir.clone());
    
    // Force detached HEAD
    Command::new("git").current_dir(&wt_dir).args(["checkout", "--detach", "HEAD"]).output().unwrap();

    let res = infer_onto(&repo);
    
    if let Ok(onto) = res {
        let head_oid = repo.resolve_commit("HEAD").unwrap();
        // If it correctly skipped Interpretation 3, it should have moved to Interpretation 4 
        // and found 'main' (which has a different OID than HEAD).
        // If it incorrectly used Interpretation 3, it will return head_oid.
        assert_ne!(onto, head_oid, "infer_onto incorrectly returned HEAD during a rebase in a worktree!");
    }
}

use git_staircase::git::GitRepo;
use git_staircase::core::{ResolvedSelector, archive_staircase, ArchiveOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_archive_staircase_leaks_worktree_detach_on_failure() {
    // ARRANGE: Create a repository with a staircase and a secondary worktree
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().join("main");
    fs::create_dir_all(&repo_dir).unwrap();
    
    // Initialize main git repo
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["init", "-b", "main"])
        .output()
        .unwrap();
    
    fs::write(repo_dir.join("file.txt"), "initial\n").unwrap();
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["add", "file.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["commit", "-m", "initial"])
        .output()
        .unwrap();
    
    // Create a branch for the staircase and add a commit
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["checkout", "-b", "feat1"])
        .output()
        .unwrap();
    fs::write(repo_dir.join("file1.txt"), "feat1\n").unwrap();
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["add", "file1.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["commit", "-m", "feat1"])
        .output()
        .unwrap();
    
    // Create a secondary worktree attached to feat1
    let wt_dir = tmp.path().join("wt1");
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["worktree", "add", wt_dir.to_str().unwrap(), "feat1"])
        .output()
        .unwrap();
    
    let repo = GitRepo::new(repo_dir.clone());
    
    // Adopt the staircase (create metadata)
    let discoveries = git_staircase::core::discovery::discover(&repo, Some("main"), None, false).unwrap();
    let discovery = discoveries.into_iter().next().expect("Should find feat1");
    let linear = match discovery {
        git_staircase::model::Discovery::Linear(m) => m,
        _ => panic!("Expected linear discovery"),
    };
    let managed_meta = git_staircase::core::adopt(&repo, &linear).unwrap();
    let rs = git_staircase::core::resolution::resolve_by_id(&repo, &managed_meta.id).unwrap();
    let selector = ResolvedSelector { staircase: rs, step_index: None };
    
    // ARRANGE: Make the record ref immutable or cause a collision to force plan.publish to fail
    let record_ref = git_staircase::core::refs::StaircaseRefs::record(&managed_meta.id, git_staircase::model::LifecycleState::Active);
    
    // Update the record ref so publish() fails (ConcurrentRecordUpdate)
    std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["update-ref", &record_ref, "HEAD"])
        .output()
        .unwrap();
    
    // ACT: Call archive_staircase
    let options = ArchiveOptions {
        dry_run: false,
        ..Default::default()
    };
    let result = archive_staircase(&repo, &selector, &options);
    
    // ASSERT: It should have failed
    assert!(result.is_err(), "Archiving should have failed due to concurrent update");
    
    // ASSERT: Verify the secondary worktree state. 
    // It should have been restored to feat1, but the bug is that it's left detached.
    let wt_status = std::process::Command::new("git")
        .current_dir(&repo_dir)
        .args(&["worktree", "list", "--porcelain"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&wt_status.stdout);
    
    // Find wt1 in the output
    let wt1_info = stdout.split("\n\n").find(|block| block.contains(wt_dir.to_str().unwrap())).unwrap();
    assert!(!wt1_info.contains("detached"), "Worktree should NOT be left detached on failure: {}", wt1_info);
}

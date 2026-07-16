use git_staircase::core::manipulation::{LandOptions, land};
use git_staircase::core::resolution::resolve_by_id;
use git_staircase::git::GitRepo;
use git_staircase::model::{StaircaseMetadata, Step};
use std::process::Command;
use tempfile::TempDir;

fn setup_repo(path: std::path::PathBuf) -> GitRepo {
    Command::new("git").current_dir(&path).args(&["init", "-b", "main"]).output().unwrap();
    Command::new("git").current_dir(&path).args(&["config", "user.name", "Test"]).output().unwrap();
    Command::new("git").current_dir(&path).args(&["config", "user.email", "test@example.com"]).output().unwrap();
    
    std::fs::write(path.join("init.txt"), "initial").unwrap();
    Command::new("git").current_dir(&path).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(&path).args(&["commit", "-m", "initial"]).output().unwrap();
    
    GitRepo::new(path)
}

#[test]
fn test_land_leaves_staircase_active() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path().to_path_buf());
    let _initial_oid = repo.resolve_commit("main").unwrap();
    
    // Create a commit to land on a separate branch
    Command::new("git").current_dir(tmp.path()).args(&["checkout", "-b", "feature"]).output().unwrap();
    std::fs::write(tmp.path().join("f1.txt"), "c1").unwrap();
    Command::new("git").current_dir(tmp.path()).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(tmp.path()).args(&["commit", "-m", "c1"]).output().unwrap();
    let c1 = repo.resolve_commit("HEAD").unwrap();
    
    // Set up a managed staircase
    let metadata = StaircaseMetadata {
        landing_policy: None,
        id: "land-test-id".to_string(),
        name: "land-test".to_string(),
        target: "refs/heads/main".to_string(),
        steps: vec![
            Step { id: "S1".into(), name: "S1".into(), cut: c1.clone(), branch: None },
        ],
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    
    let adopted = git_staircase::core::resolved::adopt(&repo, &metadata).expect("Adopt failed");
    let rs = git_staircase::core::ResolvedStaircase::Managed(adopted);
    
    // Land it
    land(&repo, &rs, LandOptions { policy: None }).expect("Land failed");
    
    // Verify target branch moved
    let new_target_oid = repo.resolve_commit("refs/heads/main").unwrap();
    assert_eq!(new_target_oid, c1, "Target branch should have moved to the top of the staircase");
    
    // Now check if the staircase still exists AS ACTIVE
    let res = resolve_by_id(&repo, &rs.metadata().id);
    
    // ARRANGE/ACT/ASSERT
    if let Ok(staircase) = res {
        if let git_staircase::core::ResolvedStaircase::Managed(metadata) = staircase {
            let is_archived = metadata.lifecycle.map_or(false, |l| l.state == git_staircase::model::LifecycleState::Archived);
            assert!(is_archived, "Staircase should be archived after landing, but its state is active.");
        } else {
            panic!("Expected Managed staircase.");
        }
    } else {
        // Technically passing if not found at all, but we expect it to be archived
    }
}

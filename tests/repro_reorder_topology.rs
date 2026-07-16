use git_staircase::core::manipulation::{ReorderOptions, reorder};
use git_staircase::core::resolved::{adopt, is_clean};
use git_staircase::core::resolution::resolve_by_id;
use git_staircase::git::GitRepo;
use git_staircase::model::{StaircaseMetadata, Step};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn setup_repo() -> (TempDir, GitRepo) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    Command::new("git").current_dir(&path).args(&["init", "-b", "main"]).output().unwrap();
    
    // Initial commit
    fs::write(path.join("init.txt"), "initial").unwrap();
    Command::new("git").current_dir(&path).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(&path).args(&["commit", "-m", "initial"]).output().unwrap();
    
    let repo = GitRepo::new(path);
    (tmp, repo)
}

#[test]
fn test_reorder_topology_invalidation() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;
    let initial_oid = repo.resolve_commit("main").unwrap();
    
    // Create C1 and C2
    fs::write(dir.join("f1.txt"), "c1").unwrap();
    Command::new("git").current_dir(dir).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(dir).args(&["commit", "-m", "c1"]).output().unwrap();
    let c1 = repo.resolve_commit("HEAD").unwrap();
    
    fs::write(dir.join("f2.txt"), "c2").unwrap();
    Command::new("git").current_dir(dir).args(&["add", "."]).output().unwrap();
    Command::new("git").current_dir(dir).args(&["commit", "-m", "c2"]).output().unwrap();
    let c2 = repo.resolve_commit("HEAD").unwrap();
    
    // Create metadata manually
    let metadata = StaircaseMetadata {
        landing_policy: None,
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: initial_oid,
        steps: vec![
            Step { id: "S1".into(), name: "S1".into(), cut: c1.clone(), branch: None },
            Step { id: "S2".into(), name: "S2".into(), cut: c2.clone(), branch: None },
        ],
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    
    // Adopt it
    let adopted = adopt(&repo, &metadata).expect("Failed to adopt");
    let rs = git_staircase::core::ResolvedStaircase::Managed(adopted);
    assert!(is_clean(&repo, rs.metadata()).unwrap());

    // Reorder S2 before S1 with no_restack: true
    reorder(&repo, &rs, &[1, 0], ReorderOptions { no_restack: true }).expect("Reorder failed");
    
    let rs_after = resolve_by_id(&repo, &rs.metadata().id).unwrap();
    let clean = is_clean(&repo, rs_after.metadata()).unwrap();
    
    // If we reached here, reorder succeeded in creating an invalid state.
    assert!(!clean, "Staircase should be invalid after reorder [1, 0]");
}

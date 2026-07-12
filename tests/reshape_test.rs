mod common;
use common::*;
use git_staircase::core;
use std::fs;

#[test]
fn test_reorder() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create three steps: main -> step1 -> step2 -> step3
    run_git(dir, &["checkout", "-b", "step1"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    let _c2 = commit(dir, "file2.txt", "2", "commit 2");
    run_git(dir, &["checkout", "-b", "step3"]);
    let _c3 = commit(dir, "file3.txt", "3", "commit 3");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");

    // Reorder: 1, 3, 2
    core::reorder(
        &repo,
        &rs,
        &[0, 2, 1],
        core::ReorderOptions { no_restack: false },
    )
    .expect("Reorder failed");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");
    let status = core::get_status_metadata(&repo, rs.metadata().clone(), !rs.is_managed()).unwrap();
    assert_eq!(status.metadata.steps.len(), 3);

    // Expected order: step1, step3, step2
    assert_eq!(status.metadata.steps[0].name, "step1");
    assert_eq!(status.metadata.steps[1].name, "step3");
    assert_eq!(status.metadata.steps[2].name, "step2");

    let new_c1 = &status.metadata.steps[0].cut;
    let new_c3 = &status.metadata.steps[1].cut;
    let new_c2 = &status.metadata.steps[2].cut;

    // c1 should remain the same as it's the first in reorder and it was first originally
    assert_eq!(new_c1, &c1);

    // Check ancestry: main -> new_c1 -> new_c3 -> new_c2
    let main_oid = repo.resolve_ref("main").unwrap();
    assert!(repo.is_ancestor(&main_oid, new_c1).unwrap());
    assert!(repo.is_ancestor(new_c1, new_c3).unwrap());
    assert!(repo.is_ancestor(new_c3, new_c2).unwrap());

    // Verify branch tips are updated
    assert_eq!(repo.resolve_ref("refs/heads/step1").unwrap(), *new_c1);
    assert_eq!(repo.resolve_ref("refs/heads/step3").unwrap(), *new_c3);
    assert_eq!(repo.resolve_ref("refs/heads/step2").unwrap(), *new_c2);

    // Verify file contents at the top
    run_git(dir, &["checkout", "step2"]);
    assert_eq!(fs::read_to_string(dir.join("file1.txt")).unwrap(), "1");
    assert_eq!(fs::read_to_string(dir.join("file2.txt")).unwrap(), "2");
    assert_eq!(fs::read_to_string(dir.join("file3.txt")).unwrap(), "3");
}

#[test]
fn test_drop() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create three steps: main -> step1 -> step2 -> step3
    run_git(dir, &["checkout", "-b", "step1"]);
    commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    commit(dir, "file2.txt", "2", "commit 2");
    run_git(dir, &["checkout", "-b", "step3"]);
    commit(dir, "file3.txt", "3", "commit 3");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");

    // Drop step 2
    core::drop(
        &repo,
        &rs,
        1,
        core::DropOptions {
            restack: true,
            leave_descendants_stale: false,
        },
    )
    .expect("Drop failed");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");
    let status = core::get_status_metadata(&repo, rs.metadata().clone(), !rs.is_managed()).unwrap();
    assert_eq!(status.metadata.steps.len(), 2);
    assert_eq!(status.metadata.steps[0].name, "step1");
    assert_eq!(status.metadata.steps[1].name, "step3");

    let new_c1 = &status.metadata.steps[0].cut;
    let new_c3 = &status.metadata.steps[1].cut;

    // Check ancestry: main -> new_c1 -> new_c3
    let main_oid = repo.resolve_ref("main").unwrap();
    assert!(repo.is_ancestor(&main_oid, new_c1).unwrap());
    assert!(repo.is_ancestor(new_c1, new_c3).unwrap());

    // Verify file contents
    run_git(dir, &["checkout", "step3"]);
    assert_eq!(fs::read_to_string(dir.join("file1.txt")).unwrap(), "1");
    assert!(dir.join("file2.txt").exists() == false);
    assert_eq!(fs::read_to_string(dir.join("file3.txt")).unwrap(), "3");
}

#[test]
fn test_move() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create two steps: main -> step1 -> step2
    run_git(dir, &["checkout", "-b", "step1"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    let c2_1 = commit(dir, "file2_1.txt", "2.1", "commit 2.1");
    let _c2_2 = commit(dir, "file2_2.txt", "2.2", "commit 2.2");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");

    // Move c2_1 from step 2 to step 1
    core::move_commits(&repo, &rs, 1, 0, &[c2_1.clone()]).expect("Move failed");

    let rs = core::resolve_staircase(&repo, "step", Some("main"))
        .unwrap()
        .expect("Staircase found");
    let status = core::get_status_metadata(&repo, rs.metadata().clone(), !rs.is_managed()).unwrap();
    assert_eq!(status.metadata.steps.len(), 2);
    assert_eq!(status.metadata.steps[0].name, "step1");
    assert_eq!(status.metadata.steps[1].name, "step2");

    let new_c1 = &status.metadata.steps[0].cut;
    let new_c2 = &status.metadata.steps[1].cut;

    assert_eq!(new_c1, &c2_1);

    // Check ancestry: main -> c1 -> new_c1 (c2_1) -> new_c2
    let main_oid = repo.resolve_ref("main").unwrap();
    assert!(repo.is_ancestor(&main_oid, &c1).unwrap());
    assert!(repo.is_ancestor(&c1, new_c1).unwrap());
    assert!(repo.is_ancestor(new_c1, new_c2).unwrap());

    // Verify file contents
    run_git(dir, &["checkout", "step1"]);
    assert!(dir.join("file2_1.txt").exists());
    assert!(dir.join("file2_2.txt").exists() == false);
}

#[test]
fn test_reorder_without_branches() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;
    use git_staircase::model::{StaircaseMetadata, Step};
    use uuid::Uuid;

    run_git(dir, &["checkout", "-b", "step1"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");
    run_git(dir, &["checkout", "-b", "step3"]);
    let c3 = commit(dir, "file3.txt", "3", "commit 3");

    let metadata = StaircaseMetadata {
            landing_policy: None,
        id: Uuid::new_v4().to_string(),
        name: "mystaircase".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: String::new(),
                name: "step1".to_string(),
                cut: c1.clone(),
                branch: Some("step1".to_string()),
            },
            Step {
                id: String::new(),
                name: "step2".to_string(),
                cut: c2.clone(),
                branch: Some("step2".to_string()),
            },
            Step {
                id: String::new(),
                name: "step3".to_string(),
                cut: c3.clone(),
                branch: Some("step3".to_string()),
            },
        ],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };

    core::adopt(&repo, &metadata).expect("Adoption failed");

    // Delete branches step1 and step2, keeping only step3 (the top)
    run_git(dir, &["branch", "-D", "step1"]);
    run_git(dir, &["branch", "-D", "step2"]);

    let rs = core::resolve_staircase(&repo, "mystaircase", None)
        .expect("Resolve failed")
        .expect("Staircase not found");

    // Reorder: 1, 3, 2
    core::reorder(
        &repo,
        &rs,
        &[0, 2, 1],
        core::ReorderOptions { no_restack: false },
    )
    .expect("Reorder failed");

    let rs = core::resolve_staircase(&repo, "mystaircase", None)
        .expect("Resolve failed")
        .expect("Staircase not found");
    let status = core::get_status_metadata(&repo, rs.metadata().clone(), !rs.is_managed()).unwrap();

    assert_eq!(status.metadata.steps.len(), 3);
    assert_eq!(status.metadata.steps[0].name, "step1");
    assert_eq!(status.metadata.steps[1].name, "step3");
    assert_eq!(status.metadata.steps[2].name, "step2");

    // Verify ancestry
    let main_oid = repo.resolve_ref("main").unwrap();
    assert!(
        repo.is_ancestor(&main_oid, &status.metadata.steps[0].cut)
            .unwrap()
    );
    assert!(
        repo.is_ancestor(&status.metadata.steps[0].cut, &status.metadata.steps[1].cut)
            .unwrap()
    );
    assert!(
        repo.is_ancestor(&status.metadata.steps[1].cut, &status.metadata.steps[2].cut)
            .unwrap()
    );

    // Verify step refs are updated
    assert_eq!(
        repo.resolve_ref(&format!("refs/staircase-state/{}/steps/step1", metadata.id))
            .unwrap(),
        status.metadata.steps[0].cut
    );
    assert_eq!(
        repo.resolve_ref(&format!("refs/staircase-state/{}/steps/step3", metadata.id))
            .unwrap(),
        status.metadata.steps[1].cut
    );
    assert_eq!(
        repo.resolve_ref(&format!("refs/staircase-state/{}/steps/step2", metadata.id))
            .unwrap(),
        status.metadata.steps[2].cut
    );
}

#[test]
fn test_restack_without_branches() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;
    use git_staircase::model::{StaircaseMetadata, Step};
    use uuid::Uuid;

    run_git(dir, &["checkout", "-b", "step1"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");
    run_git(dir, &["checkout", "-b", "step2"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let metadata = StaircaseMetadata {
            landing_policy: None,
        id: Uuid::new_v4().to_string(),
        name: "mystaircase".to_string(),
        target: "main".to_string(),
        steps: vec![
            Step {
                id: String::new(),
                name: "step1".to_string(),
                cut: c1.clone(),
                branch: Some("step1".to_string()),
            },
            Step {
                id: String::new(),
                name: "step2".to_string(),
                cut: c2.clone(),
                branch: Some("step2".to_string()),
            },
        ],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
    };

    core::adopt(&repo, &metadata).expect("Adoption failed");

    // Amend step 1 branch
    run_git(dir, &["checkout", "step1"]);
    let c1_new = commit(dir, "file1.txt", "1.1", "commit 1 amended");

    // Delete branch step2
    run_git(dir, &["branch", "-D", "step2"]);

    let rs = core::resolve_staircase(&repo, "mystaircase", None)
        .expect("Resolve failed")
        .expect("Staircase not found");

    // ACT: restack
    core::restack(
        &repo,
        &rs,
        core::RebaseOptions {
            leave_upper_steps_stale: false,
        },
    )
    .expect("Restack failed");

    let rs = core::resolve_staircase(&repo, "mystaircase", None)
        .expect("Resolve failed")
        .expect("Staircase not found");

    assert_eq!(rs.metadata().steps[0].cut, c1_new);
    assert!(
        repo.is_ancestor(&c1_new, &rs.metadata().steps[1].cut)
            .unwrap()
    );
}

use git_staircase::GitRepo;
use git_staircase::core::persistence::record::{read_record, write_record};
use git_staircase::model::{
    LifecycleState, StaircaseLifecycle, StaircaseMetadata, StaircaseUserMetadata,
};
use tempfile::tempdir;

#[test]
fn test_record_read_failure() {
    let dir = tempdir().unwrap();
    let repo_dir = dir.path();
    std::process::Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .current_dir(repo_dir)
        .output()
        .unwrap();
    let repo = GitRepo::new(repo_dir.to_path_buf());

    // Create main branch commit
    std::fs::write(repo_dir.join("file"), "content").unwrap();
    std::process::Command::new("git")
        .arg("add")
        .arg("file")
        .current_dir(repo_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(repo_dir)
        .output()
        .unwrap();

    let metadata = StaircaseMetadata {
        id: "test-id".into(),
        name: "test-name".into(),
        target: "main".into(),
        steps: vec![],
        landing_policy: None,
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    let user_metadata = StaircaseUserMetadata::default();
    let lifecycle = StaircaseLifecycle {
        state: LifecycleState::Active,
        archive_reason: None,
        name_reserved: false,
        events: vec![],
    };

    let record = write_record(
        &repo,
        &metadata,
        &user_metadata,
        &lifecycle,
        None,
        None,
        false,
    )
    .unwrap();
    let record_oid = record.record_oid;

    // Create a ref pointing to the blob (NOT in refs/heads/)
    let ref_name = "refs/staircase/test-record";
    repo.update_ref(ref_name, &record_oid, None).unwrap();

    // Try to read by ref - this should fail because it calls resolve_commit
    let result = read_record(&repo, ref_name);
    assert!(
        result.is_err(),
        "Read record should have failed due to blob vs commit resolution mismatch. Result: {:?}",
        result
    );
}

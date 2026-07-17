use git_staircase::GitRepo;
use git_staircase::workspace::{
    BootstrapOptions, Capability, bootstrap, doctor, forget_workspace_record,
    list_workspace_records,
};
use git_staircase::workspace::storage::{StorageDirGuard, set_thread_storage_dir};
use std::fs;
use tempfile::TempDir;

fn setup_test_repo() -> (
    StorageDirGuard,
    TempDir,
    GitRepo,
    TempDir,
) {
    let repo_dir = TempDir::new().unwrap();
    let storage_dir = TempDir::new().unwrap();
    let guard = set_thread_storage_dir(storage_dir.path());

    let repo = GitRepo::new(repo_dir.path().to_path_buf());
    repo.run(&["init"]).unwrap();
    repo.run(&["config", "user.name", "Test User"]).unwrap();
    repo.run(&["config", "user.email", "test@example.com"])
        .unwrap();

    // Create initial commit
    let file_path = repo_dir.path().join("file.txt");
    fs::write(&file_path, "initial").unwrap();
    repo.run(&["add", "file.txt"]).unwrap();
    repo.run(&["commit", "-m", "initial"]).unwrap();

    (guard, repo_dir, repo, storage_dir)
}

#[test]
fn test_standalone_git_repo_bootstrap() {
    let (guard, repo_dir, repo, storage_dir) = setup_test_repo();
    let _ = (&guard, &repo_dir, &storage_dir);

    let options = BootstrapOptions::default();
    let res = bootstrap(&repo, &options).unwrap();

    assert!(res.newly_configured);
    assert!(res.message.is_some());
    let msg = res.message.unwrap();
    assert!(msg.contains("Configured Staircase workspace"));
    assert!(msg.contains("workspace: single Git repository"));

    // Check saved record
    let record = res.record;
    assert_eq!(
        record
            .capability_bindings
            .get(&Capability::Workspace)
            .unwrap()
            .provider,
        "core.git"
    );
    assert_eq!(
        record
            .capability_bindings
            .get(&Capability::ProjectMapping)
            .unwrap()
            .provider,
        "core.git"
    );

    // Second bootstrap should not report newly configured
    let res2 = bootstrap(&repo, &options).unwrap();
    assert!(!res2.newly_configured);
    assert!(res2.message.is_none());
}

#[test]
fn test_no_bootstrap_flag() {
    let (guard, repo_dir, repo, storage_dir) = setup_test_repo();
    let _ = (&guard, &repo_dir, &storage_dir);

    let options = BootstrapOptions {
        no_bootstrap: true,
        ..Default::default()
    };
    let res = bootstrap(&repo, &options).unwrap();
    assert!(!res.newly_configured);

    let records = list_workspace_records().unwrap();
    assert!(records.is_empty());
}

#[test]
fn test_workspace_mode_single_git() {
    let (guard, repo_dir, repo, storage_dir) = setup_test_repo();
    let _ = (&guard, &repo_dir, &storage_dir);

    let options = BootstrapOptions {
        workspace_mode: Some("single-git".to_string()),
        ..Default::default()
    };
    let res = bootstrap(&repo, &options).unwrap();
    assert_eq!(
        res.record
            .capability_bindings
            .get(&Capability::Workspace)
            .unwrap()
            .provider,
        "core.git"
    );
}

#[test]
fn test_provider_profile_expansion() {
    let (guard, repo_dir, repo, storage_dir) = setup_test_repo();
    let _ = (&guard, &repo_dir, &storage_dir);

    let options = BootstrapOptions {
        provider_profile: Some("repo+gerrit".to_string()),
        ..Default::default()
    };
    let res = bootstrap(&repo, &options).unwrap();

    let bindings = res.record.capability_bindings;
    assert_eq!(
        bindings.get(&Capability::Workspace).unwrap().provider,
        "repo"
    );
    assert_eq!(
        bindings.get(&Capability::Review).unwrap().provider,
        "gerrit"
    );
    assert_eq!(
        bindings.get(&Capability::Verification).unwrap().provider,
        "gerrit"
    );
}

#[test]
fn test_workspace_doctor() {
    let (guard, repo_dir, repo, storage_dir) = setup_test_repo();
    let _ = (&guard, &repo_dir, &storage_dir);

    let options = BootstrapOptions::default();
    let report = doctor(&repo, &options).unwrap();

    assert_eq!(
        report.bound_capabilities.get("workspace").unwrap(),
        "core.git"
    );
    assert!(report.installed_providers.contains(&"core.git".to_string()));
}

#[test]
fn test_detached_head_integration_context() {
    let (guard, repo_dir, repo, storage_dir) = setup_test_repo();
    let _ = (&guard, &repo_dir, &storage_dir);

    let head_oid = repo.resolve_commit("HEAD").unwrap();
    repo.run(&["checkout", &head_oid]).unwrap();

    let current_branch = repo.current_branch().unwrap();
    assert!(current_branch.is_none());

    let inferred = git_staircase::core::inference::infer_onto(&repo).unwrap();
    assert_eq!(inferred, head_oid);
}

#[test]
fn test_forget_workspace() {
    let (guard, repo_dir, repo, storage_dir) = setup_test_repo();
    let _ = (&guard, &repo_dir, &storage_dir);

    let options = BootstrapOptions::default();
    let res = bootstrap(&repo, &options).unwrap();

    let ws_id = res.record.workspace_id.clone();

    let removed = forget_workspace_record(&ws_id).unwrap();
    assert!(removed);

    let records = list_workspace_records().unwrap();
    assert!(records.is_empty());
}

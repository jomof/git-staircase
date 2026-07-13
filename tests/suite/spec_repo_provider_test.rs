use git_staircase::GitRepo;
use git_staircase::workspace::{
    bootstrap, probe_repo_workspace, BootstrapOptions, Capability,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn setup_repo_workspace() -> (
    std::sync::MutexGuard<'static, ()>,
    TempDir,
    PathBuf,
    GitRepo,
    TempDir,
) {
    let guard = TEST_MUTEX.lock().unwrap();
    let client_root_dir = TempDir::new().unwrap();
    let storage_dir = TempDir::new().unwrap();

    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_dir.path());
    }

    let client_root = client_root_dir.path().to_path_buf();
    let dot_repo = client_root.join(".repo");
    fs::create_dir_all(&dot_repo).unwrap();

    let manifest_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
  <remote name="origin" fetch=".." review="review.example.com" />
  <default revision="main" remote="origin" />
  <project name="tools/vendor/example" path="tools/example" dest-branch="main" />
</manifest>"#;

    fs::write(dot_repo.join("manifest.xml"), manifest_xml).unwrap();

    let proj_dir = client_root.join("tools").join("example");
    fs::create_dir_all(&proj_dir).unwrap();

    let repo = GitRepo::new(proj_dir.clone());
    repo.run(&["init"]).unwrap();
    repo.run(&["config", "user.name", "Test User"]).unwrap();
    repo.run(&["config", "user.email", "test@example.com"]).unwrap();

    let file_path = proj_dir.join("file.txt");
    fs::write(&file_path, "hello").unwrap();
    repo.run(&["add", "file.txt"]).unwrap();
    repo.run(&["commit", "-m", "initial"]).unwrap();

    (guard, client_root_dir, client_root, repo, storage_dir)
}

#[test]
fn test_repo_provider_discovery_and_bootstrap() {
    let (_guard, _client_root_dir, client_root, repo, _storage_dir) = setup_repo_workspace();

    let cand = probe_repo_workspace(&repo).unwrap();
    assert!(cand.is_some());
    let cand = cand.unwrap();

    assert_eq!(cand.provider, "repo");
    assert_eq!(
        cand.workspace_root,
        client_root.canonicalize().unwrap()
    );
    assert_eq!(cand.claim, "authoritative");
    assert_eq!(cand.confidence, "high");

    let current_proj = cand.current_project.unwrap();
    assert_eq!(current_proj.identity, "tools/vendor/example");
    assert_eq!(current_proj.path, PathBuf::from("tools/example"));

    // Test bootstrap selection
    let res = bootstrap(&repo, &BootstrapOptions::default()).unwrap();
    assert!(res.newly_configured);
    assert_eq!(
        res.record
            .capability_bindings
            .get(&Capability::Workspace)
            .unwrap()
            .provider,
        "repo"
    );
    assert_eq!(
        res.record.current_project_id.as_deref(),
        Some("tools/vendor/example")
    );
}

#[test]
fn test_repo_provider_outside_workspace() {
    let repo_dir = TempDir::new().unwrap();
    let repo = GitRepo::new(repo_dir.path().to_path_buf());
    repo.run(&["init"]).unwrap();

    let cand = probe_repo_workspace(&repo).unwrap();
    assert!(cand.is_none());
}

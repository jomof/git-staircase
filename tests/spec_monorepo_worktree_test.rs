use git_staircase::GitRepo;
use git_staircase::monorepo::{
    CreateWorktreeOptions, create_monorepo_worktree, find_shadow_worktree_for_path,
    load_registry, prune_monorepo_worktrees, remove_monorepo_worktree,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn setup_env() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = TEST_MUTEX.lock().unwrap();
    let registry_dir = TempDir::new().unwrap();
    let registry_file = registry_dir.path().join("worktrees.json");
    let worktrees_dir = registry_dir.path().join("worktrees");
    unsafe {
        std::env::set_var("GIT_MONOREPO_REGISTRY_PATH", &registry_file);
        std::env::set_var("GIT_MONOREPO_WORKTREES_DIR", &worktrees_dir);
    }
    (guard, registry_dir)
}

fn setup_repo_workspace() -> (TempDir, PathBuf, GitRepo) {
    let client_root_dir = TempDir::new().unwrap();
    let client_root = client_root_dir.path().canonicalize().unwrap();
    let dot_repo = client_root.join(".repo");
    fs::create_dir_all(&dot_repo).unwrap();

    let manifest_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
  <remote name="origin" fetch=".." review="review.example.com" />
  <default revision="main" remote="origin" />
  <project name="tools/vendor/example" path="tools/example" dest-branch="main" />
  <project name="common" path="common" dest-branch="main" />
</manifest>"#;
    fs::write(dot_repo.join("manifest.xml"), manifest_xml).unwrap();

    let proj_dir = client_root.join("tools").join("example");
    fs::create_dir_all(&proj_dir).unwrap();

    let repo = GitRepo::new(proj_dir.clone());
    repo.run(&["init"]).unwrap();
    repo.run(&["config", "user.name", "Test User"]).unwrap();
    repo.run(&["config", "user.email", "test@example.com"]).unwrap();
    fs::write(proj_dir.join("app.txt"), "hello").unwrap();
    repo.run(&["add", "app.txt"]).unwrap();
    repo.run(&["commit", "-m", "initial"]).unwrap();

    let common_dir = client_root.join("common");
    fs::create_dir_all(&common_dir).unwrap();
    let common_repo = GitRepo::new(common_dir.clone());
    common_repo.run(&["init"]).unwrap();
    common_repo.run(&["config", "user.name", "Test User"]).unwrap();
    common_repo.run(&["config", "user.email", "test@example.com"]).unwrap();
    fs::write(common_dir.join("common.txt"), "common content").unwrap();
    common_repo.run(&["add", "common.txt"]).unwrap();
    common_repo.run(&["commit", "-m", "initial common"]).unwrap();

    fs::write(client_root.join("WORKSPACE"), "# Bazel workspace").unwrap();

    (client_root_dir, client_root, repo)
}

#[test]
fn test_create_and_list_monorepo_worktree() {
    let (_guard, _reg_dir) = setup_env();
    let (_client_dir, client_root, repo) = setup_repo_workspace();

    let options = CreateWorktreeOptions {
        repo_paths: vec![PathBuf::from("tools/example")],
        branch: Some("shadow-branch-1".to_string()),
        commit: None,
        upstream: None,
        base: None,
        name: Some("test1".to_string()),
        custom_target_path: None,
    };

    let entry = create_monorepo_worktree(&repo, &options).unwrap();
    assert_eq!(entry.id, "shadow-test1");
    assert!(entry.path.exists());
    assert_eq!(entry.primary_root, client_root);

    // Verify Bazel .bazelrc isolation
    let bazelrc = entry.path.join(".bazelrc");
    assert!(bazelrc.exists());
    let bazelrc_content = fs::read_to_string(&bazelrc).unwrap();
    assert!(bazelrc_content.contains("startup --output_base="));

    // Verify active repo is Git worktree
    let active_worktree = entry.path.join("tools").join("example");
    assert!(active_worktree.join("app.txt").exists());
    assert!(active_worktree.join(".git").is_file()); // Git worktree .git file

    // Verify non-active directory is symlinked
    let common_symlink = entry.path.join("common");
    assert!(common_symlink.exists());

    // Verify exclusions (.repo is excluded)
    assert!(!entry.path.join(".repo").exists());

    // Check registry
    let reg = load_registry().unwrap();
    assert_eq!(reg.worktrees.len(), 1);
    assert_eq!(reg.worktrees[0].id, "shadow-test1");

    // Check shadow workspace lookup
    let shadow_lookup = find_shadow_worktree_for_path(&active_worktree).unwrap();
    assert!(shadow_lookup.is_some());
    assert_eq!(shadow_lookup.unwrap().primary_root, client_root);

    // Remove worktree
    let removed = remove_monorepo_worktree("shadow-test1", true).unwrap();
    assert!(removed);
    assert!(!entry.path.exists());

    let reg_after = load_registry().unwrap();
    assert_eq!(reg_after.worktrees.len(), 0);
}

#[test]
fn test_prune_monorepo_worktrees() {
    let (_guard, _reg_dir) = setup_env();
    let (_client_dir, _client_root, repo) = setup_repo_workspace();

    let options = CreateWorktreeOptions {
        repo_paths: vec![PathBuf::from("tools/example")],
        branch: None,
        commit: None,
        upstream: None,
        base: None,
        name: Some("prune-target".to_string()),
        custom_target_path: None,
    };

    let entry = create_monorepo_worktree(&repo, &options).unwrap();
    assert!(entry.path.exists());

    let pruned = prune_monorepo_worktrees(true, None).unwrap();
    assert_eq!(pruned, vec!["shadow-prune-target"]);
    assert!(!entry.path.exists());
}

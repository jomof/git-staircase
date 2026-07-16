use git_staircase::GitRepo;
use git_staircase::workspace::github_provider::{
    create_github_upload_plan, get_github_verification, parse_github_remote_url, probe_github_route,
};
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn setup_github_repo() -> (
    std::sync::MutexGuard<'static, ()>,
    TempDir,
    GitRepo,
    TempDir,
) {
    let guard = TEST_MUTEX.lock().unwrap();
    let repo_dir = TempDir::new().unwrap();
    let storage_dir = TempDir::new().unwrap();

    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_dir.path());
    }

    let repo = GitRepo::new(repo_dir.path().to_path_buf());
    repo.run(&["init"]).unwrap();
    repo.run(&["config", "user.name", "Test User"]).unwrap();
    repo.run(&["config", "user.email", "test@example.com"])
        .unwrap();
    repo.run(&[
        "remote",
        "add",
        "origin",
        "git@github.com:example-org/example-repo.git",
    ])
    .unwrap();

    let file_path = repo_dir.path().join("file.txt");
    fs::write(&file_path, "initial").unwrap();
    repo.run(&["add", "file.txt"]).unwrap();
    repo.run(&["commit", "-m", "initial commit"]).unwrap();

    (guard, repo_dir, repo, storage_dir)
}

#[test]
fn test_github_url_parsing() {
    let loc1 = parse_github_remote_url("https://github.com/owner/repo.git").unwrap();
    assert_eq!(loc1.installation, "github.com");
    assert_eq!(loc1.owner, "owner");
    assert_eq!(loc1.repository, "repo");

    let loc2 = parse_github_remote_url("git@github.com:owner/repo.git").unwrap();
    assert_eq!(loc2.installation, "github.com");
    assert_eq!(loc2.owner, "owner");
    assert_eq!(loc2.repository, "repo");

    let loc3 = parse_github_remote_url("ssh://git@github.com/owner/repo.git").unwrap();
    assert_eq!(loc3.installation, "github.com");
    assert_eq!(loc3.owner, "owner");
    assert_eq!(loc3.repository, "repo");
}

#[test]
fn test_github_route_probing() {
    let (_guard, _repo_dir, repo, _storage_dir) = setup_github_repo();

    let route = probe_github_route(&repo, None).unwrap();
    assert!(route.is_some());
    let r = route.unwrap();

    assert_eq!(r.installation, "github.com");
    assert_eq!(r.base_repository.full_name(), "example-org/example-repo");
    assert_eq!(r.remote_name, "origin");
}

#[test]
fn test_github_upload_plan_and_verification() {
    let (_guard, _repo_dir, repo, _storage_dir) = setup_github_repo();

    let head_oid = repo.resolve_commit("HEAD").unwrap();
    let route = probe_github_route(&repo, None).unwrap().unwrap();

    let plan_agg =
        create_github_upload_plan(&repo, &route, &[head_oid.clone()], Some("aggregate"), None).unwrap();
    assert_eq!(plan_agg.publications.len(), 1);
    assert_eq!(plan_agg.publications[0].head_branch, "staircase/aggregate");

    let plan_stacked =
        create_github_upload_plan(&repo, &route, &[head_oid.clone()], Some("stacked"), None).unwrap();
    assert_eq!(plan_stacked.publications.len(), 1);
    assert_eq!(plan_stacked.publications[0].head_branch, "staircase/step-1");

    let report = get_github_verification(&route, &plan_agg).unwrap();
    assert_eq!(report.aggregate_status, "passed");
    assert!(report.is_mergeable);
}

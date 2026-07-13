use git_staircase::GitRepo;
use git_staircase::workspace::gerrit_provider::{
    ChangeIdParseResult, create_gerrit_upload_plan, get_gerrit_verification, parse_change_ids,
    probe_gerrit_route,
};
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn setup_gerrit_repo() -> (
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
    repo.run(&["config", "gerrit.host", "review.example.com"])
        .unwrap();
    repo.run(&["config", "gerrit.project", "tools/example"])
        .unwrap();
    repo.run(&["config", "gerrit.dest-branch", "main"]).unwrap();

    let file_path = repo_dir.path().join("file.txt");
    fs::write(&file_path, "initial").unwrap();
    repo.run(&["add", "file.txt"]).unwrap();
    repo.run(&[
        "commit",
        "-m",
        "initial commit\n\nChange-Id: I1234567890123456789012345678901234567890",
    ])
    .unwrap();

    (guard, repo_dir, repo, storage_dir)
}

#[test]
fn test_change_id_parsing() {
    let msg1 = "feat: add feature\n\nChange-Id: Iabc12345678901234567890123456789012345678";
    assert_eq!(
        parse_change_ids(msg1),
        ChangeIdParseResult::Single("Iabc12345678901234567890123456789012345678".to_string())
    );

    let msg_none = "feat: add feature without trailer";
    assert_eq!(parse_change_ids(msg_none), ChangeIdParseResult::None);

    let msg_multi = "feat: add feature\n\nChange-Id: I1111111111111111111111111111111111111111\nChange-Id: I2222222222222222222222222222222222222222";
    match parse_change_ids(msg_multi) {
        ChangeIdParseResult::Multiple(ids) => {
            assert_eq!(ids.len(), 2);
        }
        _ => panic!("Expected Multiple Change-Ids"),
    }
}

#[test]
fn test_gerrit_route_probing() {
    let (_guard, _repo_dir, repo, _storage_dir) = setup_gerrit_repo();

    let route = probe_gerrit_route(&repo, None).unwrap();
    assert!(route.is_some());
    let r = route.unwrap();

    assert_eq!(r.server_id, "review.example.com");
    assert_eq!(r.project, "tools/example");
    assert_eq!(r.destination_branch, "refs/heads/main");
    assert_eq!(r.upload_ref, "refs/for/main");
}

#[test]
fn test_gerrit_upload_plan_and_verification() {
    let (_guard, _repo_dir, repo, _storage_dir) = setup_gerrit_repo();

    let head_oid = repo.resolve_commit("HEAD").unwrap();
    let route = probe_gerrit_route(&repo, None).unwrap().unwrap();

    let plan =
        create_gerrit_upload_plan(&repo, &route, &[head_oid.clone()], Some("per-commit")).unwrap();

    assert_eq!(plan.commits.len(), 1);
    assert_eq!(plan.commits[0].oid, head_oid);
    assert_eq!(
        plan.commits[0].change_id.as_deref(),
        Some("I1234567890123456789012345678901234567890")
    );
    assert!(plan.warnings.is_empty());

    let report = get_gerrit_verification(&route, &plan).unwrap();
    assert_eq!(report.aggregate_status, "passed");
    assert!(report.submittable);
    assert!(report.mergeable);
}

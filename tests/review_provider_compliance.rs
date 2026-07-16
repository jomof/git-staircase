use git_staircase::GitRepo;
use git_staircase::workspace::gerrit_provider::GerritProvider;
use git_staircase::workspace::github_provider::GitHubProvider;
use git_staircase::workspace::review_provider::ReviewProvider;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_github_provider_compliance() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Initialize repo
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["init", "-b", "main"])
        .output()
        .unwrap();

    fs::write(dir.join("root.txt"), "root").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "root.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "initial",
        ])
        .output()
        .unwrap();

    // Add a github remote
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .output()
        .unwrap();

    let repo = GitRepo::new(dir.to_path_buf());
    let provider = GitHubProvider;
    let instance = provider
        .probe(&repo, None)
        .unwrap()
        .expect("Should probe github");

    let oids = vec![repo.resolve_commit("HEAD").unwrap()];

    // Test show
    let show = instance.show(&repo, &oids, None).unwrap();
    assert_eq!(show.provider_label, "GitHub");
    assert_eq!(show.items.len(), 1);

    // Test plan
    let plan = instance.plan(&repo, &oids, None, None).unwrap();
    assert_eq!(plan.provider_label, "GitHub");

    // Test status
    let status = instance.status(&repo, &oids, None).unwrap();
    assert_eq!(status.provider_label, "GitHub");
}

#[test]
fn test_gerrit_provider_compliance() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Initialize repo
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["init", "-b", "main"])
        .output()
        .unwrap();

    fs::write(dir.join("root.txt"), "root").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "root.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "initial\n\nChange-Id: I1234567890123456789012345678901234567890",
        ])
        .output()
        .unwrap();

    // Add gerrit config
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["config", "gerrit.host", "gerrit.example.com"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["config", "gerrit.project", "my/project"])
        .output()
        .unwrap();

    let repo = GitRepo::new(dir.to_path_buf());
    let provider = GerritProvider;
    let instance = provider
        .probe(&repo, None)
        .unwrap()
        .expect("Should probe gerrit");

    let oids = vec![repo.resolve_commit("HEAD").unwrap()];

    // Test show
    let show = instance.show(&repo, &oids, None).unwrap();
    assert_eq!(show.provider_label, "Gerrit");
    assert_eq!(show.items.len(), 1);

    // Test plan
    let plan = instance.plan(&repo, &oids, None, None).unwrap();
    assert_eq!(plan.provider_label, "Gerrit");

    // Test status
    let status = instance.status(&repo, &oids, None).unwrap();
    assert_eq!(status.provider_label, "Gerrit");
}

#[test]
fn test_github_provider_open() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["init", "-b", "main"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .output()
        .unwrap();
    let repo = GitRepo::new(dir.to_path_buf());
    let provider = GitHubProvider;
    let instance = provider
        .probe(&repo, None)
        .unwrap()
        .expect("Should probe github");
    let open = instance.open(&repo, &[], None).unwrap();
    assert!(open.url.contains("github.com/owner/repo/pulls"));
}

#[test]
fn test_github_provider_create_with_record() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Initialize repo
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["init", "-b", "main"])
        .output()
        .unwrap();

    fs::write(dir.join("root.txt"), "root").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "root.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "initial",
        ])
        .output()
        .unwrap();

    // Add a github remote
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .output()
        .unwrap();

    // Add branches for discovery
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["checkout", "-b", "feature/auth-core"])
        .output()
        .unwrap();
    fs::write(dir.join("core.txt"), "core").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "core.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "core",
        ])
        .output()
        .unwrap();

    std::process::Command::new("git")
        .current_dir(dir)
        .args(["checkout", "-b", "feature/auth-ui"])
        .output()
        .unwrap();
    fs::write(dir.join("ui.txt"), "ui").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "ui.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "ui",
        ])
        .output()
        .unwrap();

    let repo = GitRepo::new(dir.to_path_buf());

    let provider = GitHubProvider;
    let instance = provider
        .probe(&repo, None)
        .unwrap()
        .expect("Should probe github");

    // Adopt and read record
    let discovered = git_staircase::core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::model::Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let staircase_metadata = git_staircase::core::adopt(&repo, &s).unwrap();
    let staircase_ref = git_staircase::core::refs::StaircaseRefs::record(
        &staircase_metadata.id,
        git_staircase::model::LifecycleState::Active,
    );
    let staircase_record =
        git_staircase::core::persistence::read_record(&repo, &staircase_ref).unwrap();

    let oids = staircase_record
        .metadata
        .steps
        .iter()
        .map(|s| s.cut.clone())
        .collect::<Vec<_>>();

    // Create mutation
    let mutation = instance
        .create(&repo, &oids, None, Some(&staircase_record))
        .unwrap();
    assert_eq!(mutation.provider_label, "GitHub");
    assert_eq!(mutation.action, "create");
    assert!(mutation.record_after.is_some());
}

#[test]
fn test_gerrit_provider_create_with_record() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Initialize repo
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["init", "-b", "main"])
        .output()
        .unwrap();

    fs::write(dir.join("root.txt"), "root").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "root.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "initial\n\nChange-Id: I1234567890123456789012345678901234567890",
        ])
        .output()
        .unwrap();

    // Add gerrit config
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["config", "gerrit.host", "gerrit.example.com"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["config", "gerrit.project", "my/project"])
        .output()
        .unwrap();

    // Add branches for discovery
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["checkout", "-b", "feature/auth-core"])
        .output()
        .unwrap();
    fs::write(dir.join("core.txt"), "core").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "core.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "core\n\nChange-Id: I1234567890123456789012345678901234567891",
        ])
        .output()
        .unwrap();

    let repo = GitRepo::new(dir.to_path_buf());

    let provider = GerritProvider;
    let instance = provider
        .probe(&repo, None)
        .unwrap()
        .expect("Should probe gerrit");

    // Adopt and read record
    let discovered = git_staircase::core::discover(&repo, Some("main"), None, false).unwrap();
    let git_staircase::model::Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let staircase_metadata = git_staircase::core::adopt(&repo, &s).unwrap();
    let staircase_ref = git_staircase::core::refs::StaircaseRefs::record(
        &staircase_metadata.id,
        git_staircase::model::LifecycleState::Active,
    );
    let staircase_record =
        git_staircase::core::persistence::read_record(&repo, &staircase_ref).unwrap();

    let oids = staircase_record
        .metadata
        .steps
        .iter()
        .map(|s| s.cut.clone())
        .collect::<Vec<_>>();

    // Create mutation
    let mutation = instance
        .create(&repo, &oids, None, Some(&staircase_record))
        .unwrap();
    assert_eq!(mutation.provider_label, "Gerrit");
    assert_eq!(mutation.action, "create");
    assert!(mutation.record_after.is_some());
}

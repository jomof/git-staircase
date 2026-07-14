use git_staircase::GitRepo;
use git_staircase::workspace::github_provider::GitHubProvider;
use git_staircase::workspace::gerrit_provider::GerritProvider;
use git_staircase::workspace::review_provider::ReviewProvider;
use tempfile::TempDir;
use std::fs;

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
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "initial"])
        .output()
        .unwrap();
        
    // Add a github remote
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["remote", "add", "origin", "https://github.com/owner/repo.git"])
        .output()
        .unwrap();

    let repo = GitRepo::new(dir.to_path_buf());
    let provider = GitHubProvider;
    let instance = provider.probe(&repo, None).unwrap().expect("Should probe github");
    
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
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "initial\n\nChange-Id: I1234567890123456789012345678901234567890"])
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
    let instance = provider.probe(&repo, None).unwrap().expect("Should probe gerrit");
    
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
    std::process::Command::new("git").current_dir(dir).args(["init", "-b", "main"]).output().unwrap();
    std::process::Command::new("git").current_dir(dir).args(["remote", "add", "origin", "https://github.com/owner/repo.git"]).output().unwrap();
    let repo = GitRepo::new(dir.to_path_buf());
    let provider = GitHubProvider;
    let instance = provider.probe(&repo, None).unwrap().expect("Should probe github");
    let open = instance.open(&repo, &[], None).unwrap();
    assert!(open.url.contains("github.com/owner/repo/pulls"));
}

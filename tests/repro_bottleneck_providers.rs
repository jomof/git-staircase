use git_staircase::GitRepo;
use std::fs;
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_bottleneck_provider_discovery() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Initialize repo
    Command::new("git").arg("init").current_dir(repo_path).output().unwrap();
    Command::new("git").arg("config").arg("user.email").arg("test@example.com").current_dir(repo_path).output().unwrap();
    Command::new("git").arg("config").arg("user.name").arg("Test User").current_dir(repo_path).output().unwrap();

    let repo = GitRepo::new(repo_path.to_path_buf());
    let options = git_staircase::workspace::BootstrapOptions::default();

    // Set a custom workspace storage dir so we don't interfere with the user
    let storage_dir = temp.path().join("storage");
    unsafe { std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_dir.to_str().unwrap()); }

    // MEASURE 1: Normal bootstrap (no providers, no existing record)
    let start = Instant::now();
    git_staircase::workspace::bootstrap(&repo, &options).unwrap();
    let duration_normal = start.elapsed();
    println!("Duration with no providers: {:?}", duration_normal);

    // Create many fake providers in a directory and set GIT_STAIRCASE_PROVIDER_DIR
    let providers_dir = temp.path().join("fake_providers");
    fs::create_dir_all(&providers_dir).unwrap();
    
    // Create a "describe" script that just prints a valid descriptor
    let descriptor = serde_json::json!({
        "protocol_version": 1,
        "name": "fake",
        "version": "1.0",
        "capabilities": [],
        "probe": {
            "passive": true,
            "network": false,
            "authenticates": false,
            "mutates_workspace": false,
            "executes_repository_hooks": false
        }
    }).to_string();

    let script_content = format!("#!/bin/sh\necho '{}'", descriptor);

    for i in 0..50 {
        let provider_path = providers_dir.join(format!("provider-{}", i));
        fs::write(&provider_path, &script_content).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&provider_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&provider_path, perms).unwrap();
        }
    }

    unsafe { std::env::set_var("GIT_STAIRCASE_PROVIDER_DIR", providers_dir.to_str().unwrap()); }
    
    // Clear storage to force discovery again
    fs::remove_dir_all(&storage_dir).unwrap();

    // MEASURE 2: Bootstrap with many providers
    let start = Instant::now();
    git_staircase::workspace::bootstrap(&repo, &options).unwrap();
    let duration_many_providers = start.elapsed();
    println!("Duration with 50 providers: {:?}", duration_many_providers);
}

use git_staircase::workspace::provider::discover_installed_providers;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[test]
fn test_discover_providers_hangs_on_slow_executable() {
    let dir = tempdir().unwrap();
    let provider_path = dir.path().join("hanging_provider");
    
    // Create a script that hangs
    fs::write(&provider_path, "#!/bin/sh\nsleep 10").unwrap();
    let mut perms = fs::metadata(&provider_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&provider_path, perms).unwrap();

    // Set the provider directory env var
    unsafe { std::env::set_var("GIT_STAIRCASE_PROVIDER_DIR", dir.path()); }

    let start = Instant::now();
    let _ = discover_installed_providers();
    let elapsed = start.elapsed();

    // If it took more than 5 seconds, it's a bug (lack of timeout)
    assert!(elapsed < Duration::from_secs(2), "Discovery took {:?}, which is too long! Should have timed out.", elapsed);
}

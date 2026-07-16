mod common;
use common::*;

#[test]
fn test_porcelain_headers_list() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let (success, stdout, stderr) = run_staircase(dir, &["list", "--porcelain"]);
    assert!(success, "list --porcelain failed: {}", stderr);

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert!(fields.len() >= 2, "Line should have at least 2 fields: {}", line);
        assert_eq!(fields[0], "staircase", "First field should be record type 'staircase': {}", line);
        assert_eq!(fields[1], "1", "Second field should be schema version '1': {}", line);
    }
}

#[test]
fn test_porcelain_headers_status() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_staircase(dir, &["adopt", "auth", "feature/auth-core"]);

    let (success, stdout, stderr) = run_staircase(dir, &["status", "auth", "--porcelain"]);
    assert!(success, "status --porcelain failed: {}", stderr);

    let mut found_staircase = false;
    let mut found_step = false;

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert!(fields.len() >= 2, "Line should have at least 2 fields: {}", line);
        let record_type = fields[0];
        assert_eq!(fields[1], "1", "Second field should be schema version '1': {}", line);
        
        if record_type == "staircase" {
            found_staircase = true;
        } else if record_type == "step" {
            found_step = true;
        }
    }

    assert!(found_staircase, "Should have found 'staircase' record in status output");
    assert!(found_step, "Should have found 'step' record in status output");
}

#[test]
fn test_porcelain_headers_error() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    let (success, _stdout, stderr) = run_staircase(dir, &["show", "non-existent", "--porcelain"]);
    assert!(!success, "show non-existent should fail");

    let error_line = stderr.lines().next().expect("Should have at least one error line in stderr");
    let fields: Vec<&str> = error_line.split('\t').collect();
    
    assert!(fields.len() >= 5, "Error line should have at least 5 fields: {}", error_line);
    assert_eq!(fields[0], "error", "First field should be 'error'");
    assert_eq!(fields[1], "1", "Second field should be version '1'");
    assert_eq!(fields[2], "\"selector-not-found\"", "Third field should be error code 'not-found'");
}

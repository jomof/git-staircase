mod common;
use common::*;

#[test]
fn test_adoption_ui() {
    let (tmp, _repo) = setup_repo();
    let dir = tmp.path();

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    commit(dir, "file2.txt", "2", "commit 2");

    let (_success, stdout, _stderr) =
        run_staircase(dir, &["id", "feature/auth", "--kind", "lineage"]);

    println!("Stdout: {}", stdout);

    // The spec says it should print "adopted implicit staircase 'feature/auth'"
    assert!(stdout.contains("adopted implicit staircase 'feature/auth'"));
}

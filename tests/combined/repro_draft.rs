use git_staircase::core::operation::{MutationPlan, abort_active};
use git_staircase::git::GitRepo;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_draft_loss_on_conflict() {
    let test_dir = std::env::current_dir()
        .unwrap()
        .join("repro_draft_dir_unique");
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).unwrap();
    }
    fs::create_dir_all(&test_dir).unwrap();

    fn run_git(dir: &PathBuf, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: {:?}", args);
    }

    run_git(&test_dir, &["init"]);
    run_git(&test_dir, &["config", "user.email", "you@example.com"]);
    run_git(&test_dir, &["config", "user.name", "Your Name"]);

    fs::write(test_dir.join("file"), "base").unwrap();
    run_git(&test_dir, &["add", "file"]);
    run_git(&test_dir, &["commit", "-m", "base"]);

    run_git(&test_dir, &["checkout", "-b", "A"]);
    fs::write(test_dir.join("file"), "A").unwrap();
    run_git(&test_dir, &["commit", "-am", "A"]);

    run_git(&test_dir, &["checkout", "main"]);
    run_git(&test_dir, &["checkout", "-b", "B"]);
    fs::write(test_dir.join("file"), "B").unwrap();
    run_git(&test_dir, &["commit", "-am", "B"]);

    // Merge A into B to cause conflict
    let status = std::process::Command::new("git")
        .args(["merge", "A"])
        .current_dir(&test_dir)
        .status()
        .unwrap();
    assert!(!status.success());

    // Add some unstaged changes as well
    fs::write(test_dir.join("unstaged"), "more changes").unwrap();

    let repo = GitRepo::new(test_dir.canonicalize().unwrap());

    // Now try to run an operation. This will capture the draft.
    let mut plan = MutationPlan::new("test", None);
    // Use an invalid OID to make update-ref fail during publish
    plan.update(
        "refs/heads/dummy",
        None,
        Some("0000000000000000000000000000000000000000".to_string()),
    );

    let _ = plan.publish(&repo, false);

    // Now abort the operation. This should restore the draft.
    abort_active(&repo).expect("Should find active operation to abort");

    // Verify status.
    let output = std::process::Command::new("git")
        .args(["status"])
        .current_dir(&repo.workdir)
        .output()
        .unwrap();
    let status_str = String::from_utf8_lossy(&output.stdout);

    assert!(
        status_str.contains("both modified"),
        "Conflict state should be preserved"
    );
    assert!(
        test_dir.join("unstaged").exists(),
        "Unstaged file should exist"
    );
    let unstaged_content = fs::read_to_string(test_dir.join("unstaged")).unwrap();
    assert_eq!(
        unstaged_content, "more changes",
        "Unstaged changes should be preserved"
    );

    // Cleanup
    fs::remove_dir_all(&test_dir).unwrap();
}

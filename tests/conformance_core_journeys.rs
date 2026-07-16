mod common;
use common::TestContext;
use std::fs;

#[test]
fn journey_1_bootstraps_repo_gerrit_and_publishes_three_reviews() {
    let ctx = TestContext::new();
    let root = ctx.path();

    // ARRANGE Create a multi-project repo workspace with Gerrit remote hints.
    fs::create_dir_all(root.join(".repo")).unwrap();
    let project_path = root.join("src").join("app");
    fs::create_dir_all(&project_path).unwrap();

    // Initialize the project repo
    let project_ctx = TestContext::new(); // Use a separate context to get a git repo
    // Copy the repo to the project path
    fs::rename(project_ctx.path().join(".git"), project_path.join(".git")).unwrap();
    // Move files too
    for entry in fs::read_dir(project_ctx.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != ".git" {
            fs::rename(entry.path(), project_path.join(entry.file_name())).unwrap();
        }
    }

    let manifest_content = r#"<manifest>
  <remote name="upstream" fetch="https://git.example/platform" review="review.example.com"/>
  <default remote="upstream" revision="main"/>
  <project name="platform/app" path="src/app" upstream="refs/heads/main"/>
</manifest>"#;
    fs::write(root.join(".repo").join("manifest.xml"), manifest_content).unwrap();

    // Create three commits in the project
    let git_dir = project_path;
    for i in 1..=3 {
        fs::write(
            git_dir.join(format!("file{}.txt", i)),
            format!("content {}", i),
        )
        .unwrap();
        common::run_git(&git_dir, &["add", "."]);
        common::run_git(&git_dir, &["commit", "-m", &format!("commit {}", i)]);
    }

    let base_oid = common::run_git(&git_dir, &["rev-parse", "HEAD~3"]);
    common::run_git(&git_dir, &["update-ref", "refs/remotes/m/main", &base_oid]);

    // ACT Run git staircase list to verify discovery of the implicit staircase.
    // We need to run it from the project directory
    let (success, stdout, stderr) = common::run_staircase(&git_dir, &["list", "--porcelain"]);

    // ASSERT Verify discovery
    assert!(
        success,
        "list failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout, stderr
    );
    assert!(
        stdout.contains("main"),
        "Should discover main branch. Output: {}",
        stdout
    );

    // Check status too
    let (success_status, stdout_status, stderr_status) =
        common::run_staircase(&git_dir, &["status", "main"]);
    assert!(
        success_status,
        "status failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout_status, stderr_status
    );
    assert!(
        stdout_status.contains("implicit"),
        "Should be implicit. Output: {}",
        stdout_status
    );

    // Create a mock Gerrit remote
    let remote_dir = root.join("gerrit_remote");
    fs::create_dir_all(&remote_dir).unwrap();
    common::run_git(&remote_dir, &["init", "--bare", "-b", "main"]);

    // Set gerrit host to the remote path
    common::run_git(
        &git_dir,
        &["config", "gerrit.host", &remote_dir.to_string_lossy()],
    );

    // ACT Run git staircase review upload to upload the reviews.
    // We expect this to fail initially because of missing Change-Id (Appendix A.2.2/A.2.3 logic)
    // Actually, in Journey 1, the developer first adopts.
    let (adopt_success, _adopt_stdout, adopt_stderr) =
        common::run_staircase(&git_dir, &["adopt", "main"]);
    assert!(adopt_success, "adopt failed: {}", adopt_stderr);

    let (success, _stdout, stderr) = common::run_staircase(&git_dir, &["review", "upload", "main"]);

    // In Journey 1, it might fail or we might need to normalize first.
    // If it fails with missing-change-id, we run normalize.
    if !success && stderr.contains("missing-change-id") {
        let (norm_success, _norm_stdout, norm_stderr) =
            common::run_staircase(&git_dir, &["normalize", "main", "--ensure-change-ids"]);
        assert!(norm_success, "normalize failed: {}", norm_stderr);

        let (success2, _stdout2, stderr2) =
            common::run_staircase(&git_dir, &["review", "upload", "main"]);
        assert!(
            success2,
            "review upload failed after normalization:\nSTDOUT: {}\nSTDERR: {}",
            _stdout2, stderr2
        );
    } else {
        assert!(
            success,
            "review upload failed:\nSTDOUT: {}\nSTDERR: {}",
            _stdout, stderr
        );
    }

    // ASSERT Verify that three Change-Id identifiers are generated and present in the commits.
    let log = common::run_git(&git_dir, &["log", "-n", "3"]);
    assert_eq!(
        log.matches("Change-Id: I").count(),
        3,
        "Should have 3 Change-Ids in log:\n{}",
        log
    );
}

#[test]
fn journey_1_amend_preserves_draft_and_review_identity_across_conflicts() {
    let ctx = TestContext::new();
    let dir = ctx.path();

    // ARRANGE: Create a staircase with two steps. Both steps have Gerrit review identities.
    // main -> s1 -> s2
    ctx.run_git(&["checkout", "-b", "s1", "main"]);
    ctx.commit("conflict.txt", "line 1\n", "Step 1\n\nChange-Id: I111\n");
    ctx.run_git(&["checkout", "-b", "s2"]);
    ctx.commit(
        "conflict.txt",
        "line 1\nline 2\n",
        "Step 2\n\nChange-Id: I222\n",
    );

    // Adopt it. We need to make sure 'main' is recognized as the anchor.
    // Discovery usually picks 'main' if it exists.
    let (success, _, stderr) = ctx.run_staircase(&["adopt", "feature", "s1", "s2"]);
    assert!(success, "adopt failed: {}", stderr);

    // Checkout Step 1 to amend it.
    ctx.run_git(&["checkout", "s1"]);

    // Add a staged change (will be amended).
    // Change line 1 in a way that conflicts with Step 2 (Step 2 expects 'line 1\n').
    fs::write(dir.join("conflict.txt"), "line 1 amended\n").unwrap();
    ctx.run_git(&["add", "conflict.txt"]);

    // Add an unstaged draft change (should be preserved).
    fs::write(dir.join("draft.txt"), "draft content\n").unwrap();

    // ACT: Amend Step 1 with the staged change.
    // This should trigger a restack of Step 2, which will conflict.
    let (success, stdout, stderr) = ctx.run_staircase(&[
        "draft",
        "materialize",
        "feature",
        "--amend",
        "--message",
        "Step 1 amended\n\nChange-Id: I111\n",
    ]);

    // ASSERT: Verify the operation pauses for conflict.
    assert!(!success, "materialize should have paused for conflict");
    assert!(
        stderr.contains("conflict") || stdout.contains("conflict"),
        "Expected conflict message. STDOUT: {}\nSTDERR: {}",
        stdout,
        stderr
    );

    // ACT: Resolve conflict and git staircase continue.
    fs::write(dir.join("conflict.txt"), "line 1 amended\nline 2\n").unwrap();
    ctx.run_git(&["add", "conflict.txt"]);

    let (success, stdout, stderr) = ctx.run_staircase(&["continue"]);
    assert!(
        success,
        "continue failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout, stderr
    );

    // ASSERT: Verify Step 1 and Step 2 still have their original review identities.
    let log = ctx.run_git(&["log", "--format=%B", "-n", "2", "s2"]);
    assert!(
        log.contains("Change-Id: I111"),
        "Step 1 should retain I111. Log:\n{}",
        log
    );
    assert!(
        log.contains("Change-Id: I222"),
        "Step 2 should retain I222. Log:\n{}",
        log
    );

    // Verify the draft change was preserved.
    assert!(
        dir.join("draft.txt").exists(),
        "draft.txt should still exist"
    );
    let draft_content = fs::read_to_string(dir.join("draft.txt")).unwrap();
    assert_eq!(draft_content, "draft content\n");

    // Verify it is still unstaged (not in the index).
    let staged = ctx.run_git(&["diff", "--cached", "--name-only"]);
    assert!(
        !staged.contains("draft.txt"),
        "draft.txt should not be staged. Staged files:\n{}",
        staged
    );
}

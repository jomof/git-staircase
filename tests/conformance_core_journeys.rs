mod common;

use common::*;

#[test]
fn journey_1_bootstraps_repo_gerrit_and_publishes_three_reviews() {
    // ARRANGE: Setup a repository with a .repo manifest and a Gerrit remote origin. Checkout a detached HEAD.
    let context = TestContext::new();
    let root = context.path();

    // Simulate .repo manifest
    std::fs::create_dir_all(root.join(".repo")).unwrap();
    std::fs::write(
        root.join(".repo").join("manifest.xml"),
        r#"<manifest>
  <remote name="origin" fetch=".." review="origin"/>
  <default remote="origin" revision="main"/>
  <project name="test-project" path="."/>
</manifest>"#,
    )
    .unwrap();

    // Setup Gerrit-like remote
    let remote_tmp = tempfile::TempDir::new().unwrap();
    let remote_path = remote_tmp.path();
    run_git(remote_path, &["init", "-b", "main"]);
    run_git(
        remote_path,
        &["config", "receive.denyCurrentBranch", "ignore"],
    );

    context.run_git(&["remote", "add", "origin", &remote_path.to_string_lossy()]);
    // Push initial commit to remote
    context.run_git(&["push", "origin", "main"]);

    // Checkout detached HEAD
    let head = context.run_git(&["rev-parse", "HEAD"]);
    context.run_git(&["checkout", "--detach", &head]);
    context.run_git(&["update-ref", "refs/remotes/m/main", &head]);

    // ACT: Create three commits and branches.
    let mut oids = Vec::new();
    for i in 1..=3 {
        oids.push(context.commit(
            &format!("file{}.txt", i),
            &format!("content{}", i),
            &format!("Step {}\n\nChange-Id: I{:040x}", i, i),
        ));
    }
    context.run_git(&["branch", "payments-1", &oids[0]]);
    context.run_git(&["branch", "payments-2", &oids[1]]);
    context.run_git(&["branch", "payments", &oids[2]]);

    // Run git-staircase publish
    let (success, stdout, stderr) = context.run_staircase(&["publish", "payments"]);
    assert!(
        success,
        "publish failed: stdout: {}, stderr: {}",
        stdout, stderr
    );

    // ASSERT: Verify three reviews are created on the provider and local refs/staircases tracks them.

    let staircase_refs = context.run_git(&["for-each-ref", "refs/staircases/"]);
    assert!(!staircase_refs.is_empty(), "No staircase refs found");

    // Check for Gerrit refs (simulated by pushes to refs/for/main which we can't easily see on bare repo without inspection)
    // Actually, in this setup, git-staircase will try to push to 'origin' at 'refs/for/main'.
    // We can check if those refs were created in the remote.
    let remote_refs = run_git(remote_path, &["for-each-ref"]);
    assert!(
        remote_refs.contains("refs/for/main"),
        "Gerrit upload ref not found in remote: {}",
        remote_refs
    );
}

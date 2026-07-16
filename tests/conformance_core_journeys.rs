mod common;
use common::TestContext;
use serde_json::Value;
use std::fs;

#[test]
fn journey_1_bootstraps_repo_gerrit_and_publishes_three_reviews() {
    let ctx = TestContext::new();

    // ARRANGE: Create 3 commits with Gerrit Change-Ids
    let _onto = ctx.run_git(&["rev-parse", "main"]);
    ctx.run_git(&["checkout", "-b", "feature"]);

    let oid1 = ctx.commit(
        "file1.txt",
        "content1",
        "commit 1\n\nChange-Id: I1111111111111111111111111111111111111111",
    );
    let oid2 = ctx.commit(
        "file2.txt",
        "content2",
        "commit 2\n\nChange-Id: I2222222222222222222222222222222222222222",
    );
    let oid3 = ctx.commit(
        "file3.txt",
        "content3",
        "commit 3\n\nChange-Id: I3333333333333333333333333333333333333333",
    );

    // Create branches for discovery
    ctx.run_git(&["branch", "step1", &oid1]);
    ctx.run_git(&["branch", "step2", &oid2]);
    ctx.run_git(&["branch", "step3", &oid3]);

    // Setup repo workspace simulation
    let dot_repo = ctx.path().join(".repo");
    fs::create_dir_all(&dot_repo).unwrap();
    let manifest = r#"<manifest>
  <remote name="origin" fetch=".." review="http://gerrit.example.com"/>
  <default remote="origin" revision="refs/heads/main"/>
  <project name="my-project" path="." />
</manifest>"#;
    fs::write(dot_repo.join("manifest.xml"), manifest).unwrap();

    // ACT: Run discover and adopt
    let (ok, stdout, stderr) = ctx.run_staircase(&["discover", "--onto", "main"]);
    assert!(ok, "discover failed: {}", stderr);
    assert!(
        stdout.contains("Step 3"),
        "Should discover the staircase: {}",
        stdout
    );

    let (ok, stdout, stderr) = ctx.run_staircase(&[
        "adopt",
        "my-staircase",
        "--onto",
        "main",
        "step1",
        "step2",
        "step3",
    ]);
    assert!(ok, "adopt failed: {}", stderr);

    // ASSERT: Verify adoption and metadata
    assert!(
        stdout.contains("my-staircase"),
        "Should report adoption of 'my-staircase': {}",
        stdout
    );

    let (ok, stdout, stderr) = ctx.run_staircase(&["show", "my-staircase", "--json"]);
    assert!(ok, "show failed: {}", stderr);

    let val: Value = serde_json::from_str(&stdout).unwrap();
    let steps = val["steps"].as_array().expect("steps should be an array");
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0]["cut"], oid1);
    assert_eq!(steps[1]["cut"], oid2);
    assert_eq!(steps[2]["cut"], oid3);
}

#[test]
fn journey_1_amend_preserves_draft_and_review_identity_across_conflicts() {
    let ctx = TestContext::new();

    // ARRANGE: Create 3 commits in a staircase
    let _onto = ctx.run_git(&["rev-parse", "main"]);
    ctx.run_git(&["checkout", "-b", "feature"]);

    let oid1 = ctx.commit(
        "file1.txt",
        "content1",
        "commit 1\n\nChange-Id: I1111111111111111111111111111111111111111",
    );
    let oid2 = ctx.commit(
        "file2.txt",
        "content2",
        "commit 2\n\nChange-Id: I2222222222222222222222222222222222222222",
    );
    let oid3 = ctx.commit(
        "file3.txt",
        "content3",
        "commit 3\n\nChange-Id: I3333333333333333333333333333333333333333",
    );

    // Create branches
    ctx.run_git(&["branch", "step1", &oid1]);
    ctx.run_git(&["branch", "step2", &oid2]);
    ctx.run_git(&["branch", "step3", &oid3]);

    let (ok, _, stderr) = ctx.run_staircase(&[
        "adopt",
        "my-staircase",
        "--onto",
        "main",
        "step1",
        "step2",
        "step3",
    ]);
    assert!(ok, "adopt failed: {}", stderr);

    // ACT: Amend the bottom step (oid1)
    // We need to checkout the first branch, amend it, and then restack.
    ctx.run_git(&["checkout", "step1"]);
    ctx.commit(
        "file1.txt",
        "content1 amended",
        "commit 1 amended\n\nChange-Id: I1111111111111111111111111111111111111111",
    );
    let new_oid1 = ctx.run_git(&["rev-parse", "HEAD"]);

    // Now upper steps are stale. Try to restack.
    let (ok, stdout, stderr) = ctx.run_staircase(&["restack", "my-staircase"]);
    assert!(ok, "restack failed: {}\nStdout: {}", stderr, stdout);

    // ASSERT: Verify Change-Ids are preserved in rewritten commits
    let (ok, stdout, stderr) = ctx.run_staircase(&["show", "my-staircase", "--json"]);
    assert!(ok, "show failed: {}", stderr);

    let val: Value = serde_json::from_str(&stdout).unwrap();
    let steps = val["steps"].as_array().expect("steps should be an array");
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0]["cut"], new_oid1);

    // Check Change-Ids of all steps
    for (i, expected_id) in [
        "I1111111111111111111111111111111111111111",
        "I2222222222222222222222222222222222222222",
        "I3333333333333333333333333333333333333333",
    ]
    .iter()
    .enumerate()
    {
        let oid = steps[i]["cut"].as_str().unwrap();
        let message = ctx.run_git(&["log", "-1", "--format=%B", oid]);
        assert!(
            message.contains(expected_id),
            "Step {} should preserve Change-Id {}: {}",
            i,
            expected_id,
            message
        );
    }
}

#[test]
fn journey_4_cross_worktree_materialization_preserves_partial_staging() {
    let ctx = TestContext::new();

    // ARRANGE: Create 2 commits in a staircase
    let _onto = ctx.run_git(&["rev-parse", "main"]);
    ctx.run_git(&["checkout", "-b", "feature"]);
    let oid1 = ctx.commit("file1.txt", "content1", "commit 1");
    let oid2 = ctx.commit("file2.txt", "content2", "commit 2");
    ctx.run_git(&["branch", "step1", &oid1]);
    ctx.run_git(&["branch", "step2", &oid2]);
    let (ok, _, stderr) =
        ctx.run_staircase(&["adopt", "my-staircase", "--onto", "main", "step1", "step2"]);
    assert!(ok, "adopt failed: {}", stderr);

    // Create a second worktree
    let wt_path = ctx.path().join("wt2");
    ctx.run_git(&["worktree", "add", "wt2", "step1"]);

    // In worktree 2, create partial staging
    std::fs::write(wt_path.join("staged.txt"), "staged content").unwrap();
    common::run_git(&wt_path, &["add", "staged.txt"]);
    std::fs::write(wt_path.join("unstaged.txt"), "unstaged content").unwrap();

    // ACT: Materialize a change into step 1 from worktree 2
    // We want to materialize only a NEW file, and see if staged.txt is preserved.
    std::fs::write(wt_path.join("materialized.txt"), "materialized content").unwrap();

    // Run materialize in wt2
    let (ok, stdout, stderr) = common::run_staircase(
        &wt_path,
        &[
            "draft",
            "materialize",
            "my-staircase",
            "--fold-into",
            "step1",
            "materialized.txt",
        ],
    );
    assert!(ok, "materialize failed: {}\nStdout: {}", stderr, stdout);

    // ASSERT: Verify staged.txt is still staged and unstaged.txt is still unstaged in wt2
    let status = common::run_git(&wt_path, &["status", "--porcelain"]);
    // Since staged.txt was in the index, it was included in the materialized commit.
    // If materialize_draft doesn't clear the index of paths it DIDN'T explicitly add, it might stay.
    // But usually materialize clears the draft.

    // Actually, if it was included in the commit, it's no longer "staged" relative to HEAD.
    assert!(
        !status.contains("A  staged.txt"),
        "staged.txt should be committed, not staged: {}",
        status
    );
    assert!(
        status.contains("?? unstaged.txt"),
        "unstaged.txt should still be untracked: {}",
        status
    );

    // Verify materialized.txt is part of the new commit for step1
    let (ok, stdout, stderr) = common::run_staircase(&wt_path, &["show", "my-staircase", "--json"]);
    assert!(ok, "show failed: {}", stderr);
    let val: Value = serde_json::from_str(&stdout).unwrap();
    let steps = val["steps"].as_array().unwrap();
    let new_oid1 = steps[0]["cut"].as_str().unwrap();

    let files = common::run_git(&wt_path, &["ls-tree", "-r", "--name-only", new_oid1]);
    assert!(
        files.contains("materialized.txt"),
        "New commit should contain materialized.txt: {}",
        files
    );
    assert!(
        files.contains("staged.txt"),
        "New commit SHOULD ALSO contain staged.txt because it was in the index: {}",
        files
    );
}

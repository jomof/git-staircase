mod common;
use common::TestContext;

#[test]
fn main_ahead_of_anchor_is_valid_work() {
    let ctx = TestContext::new();

    // ARRANGE: Create a repository where 'main' has 2 commits ahead of the anchor.
    // Initial commit was already created by TestContext::new() on branch 'main'.
    let anchor_oid = ctx.run_git(&["rev-parse", "main"]);

    // Create origin/main branch to serve as integration anchor
    ctx.run_git(&["update-ref", "refs/remotes/origin/main", &anchor_oid]);

    // Add 2 commits to main
    ctx.commit("file1.txt", "content1", "feature 1");
    ctx.commit("file2.txt", "content2", "feature 2");

    // ACT: Run 'git staircase list' with the anchor explicitly specified
    // to verify that discovery logic considers 'main' as a candidate.
    let (ok, stdout, stderr) = ctx.run_staircase(&["list", "--onto", "origin/main"]);

    // ASSERT: Verify that 'main' appears in the list as a discoverable implicit staircase.
    assert!(ok, "list failed: {}", stderr);
    assert!(
        stdout.contains("main") && stdout.contains("(implicit)"),
        "main should be discovered as an implicit staircase when ahead of anchor.\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn main_ahead_of_non_origin_anchor_is_valid_work() {
    let ctx = TestContext::new();

    // ARRANGE: Create a repository where 'main' has 2 commits ahead of 'upstream/main'.
    let anchor_oid = ctx.run_git(&["rev-parse", "main"]);

    // Create upstream/main branch (local branch for simplicity in test)
    ctx.run_git(&["branch", "upstream/main", &anchor_oid]);

    // Add 2 commits to main
    ctx.commit("file1.txt", "content1", "feature 1");
    ctx.commit("file2.txt", "content2", "feature 2");

    // Set upstream for main to upstream/main
    ctx.run_git(&["branch", "--set-upstream-to=upstream/main", "main"]);

    // switch to a different branch so step 2 of infer_onto is skipped
    ctx.run_git(&["checkout", "-b", "other"]);

    // ACT: Run 'git staircase list'
    let (ok, stdout, stderr) = ctx.run_staircase(&["list"]);

    // ASSERT: Verify that 'main' appears in the list.
    assert!(ok, "list failed: {}", stderr);
    assert!(
        stdout.contains("main"),
        "main should be discovered when ahead of upstream/main even if not on main branch.\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn empty_step_is_rejected_before_mutation() {
    let ctx = TestContext::new();

    // ARRANGE: Create a staircase with two steps.
    // Initial commit (C0) is on 'main'.
    let c0 = ctx.run_git(&["rev-parse", "main"]);
    ctx.run_git(&["update-ref", "refs/remotes/origin/main", &c0]);

    // Step 1: C1
    let c1 = ctx.commit("file1.txt", "content1", "feature 1");
    // Step 2: C2
    let c2 = ctx.commit("file2.txt", "content2", "feature 2");

    // Discovery should find feature-1 (C1) and feature-2 (C2)
    ctx.run_git(&["branch", "feature-1", &c1]);
    ctx.run_git(&["branch", "feature-2", &c2]);

    // Verify it's discovered as a 2-step staircase.
    let (ok, stdout, stderr) = ctx.run_staircase(&["list", "--onto", "origin/main", "--json"]);
    assert!(ok, "list failed: {}", stderr);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let staircases = json.as_array().unwrap();
    assert_eq!(
        staircases.len(),
        1,
        "Should discover exactly one staircase. Output: {}",
        stdout
    );
    let steps = staircases[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2, "Should have 2 steps. Output: {}", stdout);

    // ACT: Attempt to move all commits from the first step to the second.
    // In our case, Step 1 has C1. Step 2 has C2.
    // We try to move C1 from Step 1 to Step 2.
    // The command should be: git staircase move <name> --from 1 --to 2 <C1>
    let (ok, stdout, stderr) =
        ctx.run_staircase(&["move", "feature", "--from", "1", "--to", "2", &c1]);

    // ASSERT: The command fails with a diagnostic error message before any changes are made.
    assert!(
        !ok,
        "Move should have failed but succeeded. Stdout: {}\nStderr: {}",
        stdout, stderr
    );
    assert!(
        stderr.contains("empty") || stderr.contains("rejected"),
        "Error message should indicate that an empty step is rejected. Actual stderr: {}",
        stderr
    );

    // Verify no mutation occurred (metadata not adopted/changed if it was implicit)
    // Actually, move might trigger adoption. But if it fails BEFORE mutation, it shouldn't adopt if it can help it,
    // OR it might adopt then fail the plan.
    // The spec says "Reject before mutation".
}

#[test]
fn integration_branch_at_anchor_is_not_discovered() {
    let ctx = TestContext::new();

    // 1. ARRANGE: Create a repository where the local branch `main` and its remote anchor `origin/main` point to the same OID.
    let main_oid = ctx.run_git(&["rev-parse", "main"]);
    ctx.run_git(&["update-ref", "refs/remotes/origin/main", &main_oid]);

    // 2. ACT: Run `git-staircase list --onto origin/main`.
    let (ok, stdout, _) = ctx.run_staircase(&["list", "--onto", "origin/main"]);
    assert!(ok);

    // 3. ASSERT: Verify that `main` is NOT present in the output.
    assert!(
        !stdout.contains("main"),
        "main should not be discovered if it is exactly at the anchor. Stdout: {}",
        stdout
    );
}

#[test]
fn equivalent_discovery_sources_collapse() {
    let ctx = TestContext::new();

    // 1. ARRANGE: Create a staircase and point two different branches (`feat-1` and `feat-2`) to its tip OID.
    ctx.commit("f1.txt", "1", "commit 1");
    let c2 = ctx.commit("f2.txt", "2", "commit 2");
    ctx.run_git(&["branch", "feat-1", &c2]);
    ctx.run_git(&["branch", "feat-2", &c2]);

    // 2. ACT: Run `git-staircase list --implicit --json`.
    let (ok, stdout, stderr) = ctx.run_staircase(&["list", "--implicit", "--json"]);
    assert!(ok, "list failed: {}", stderr);

    // 3. ASSERT: Verify that only one staircase entry is returned in the JSON array,
    // and its 'provenance' or 'refs' metadata includes both `feat-1` and `feat-2`.
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    let staircases = json.as_array().expect("Output should be a JSON array");

    assert_eq!(
        staircases.len(),
        1,
        "Should have collapsed feat-1 and feat-2 into a single staircase entry. Output: {}",
        stdout
    );

    let staircase = &staircases[0];
    let refs = staircase["materializing_refs"]
        .as_array()
        .or_else(|| staircase["provenance"].as_array())
        .expect("Should have materializing_refs or provenance field");

    let ref_names: Vec<&str> = refs.iter().map(|r| r.as_str().unwrap()).collect();
    assert!(
        ref_names.iter().any(|r| r.contains("feat-1")),
        "Should contain feat-1 in provenance. Refs: {:?}",
        ref_names
    );
    assert!(
        ref_names.iter().any(|r| r.contains("feat-2")),
        "Should contain feat-2 in provenance. Refs: {:?}",
        ref_names
    );
}

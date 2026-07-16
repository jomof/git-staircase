mod common;

use common::*;

#[test]
fn observation_never_adopts() {
    // ARRANGE: Setup a repository with an implicit staircase.
    // An implicit staircase is a branch ahead of the integration anchor (main).
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "feature"]);
    context.commit("feature.txt", "feature content", "feature commit");

    // Verify it is discovered as implicit.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(stdout.contains("(implicit)"));
    assert!(stdout.contains("feature"));

    // ACT: Run exhaustive inspection commands.
    let commands = [
        vec!["list"],
        vec!["show", "feature"],
        vec!["status", "feature"],
        vec!["steps", "feature"],
        vec!["commits", "feature"],
        vec!["discover"],
        vec!["log", "feature"],
        vec!["diff", "feature"],
        vec!["describe", "feature"],
        vec!["id", "feature"],
        vec!["rev-parse", "feature"],
    ];

    for args in commands {
        println!("Running: git-staircase {:?}", args);
        context.run_staircase(&args);

        let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
        for line in refs.lines() {
            assert!(
                !line.starts_with("refs/staircases/"),
                "Found unexpected staircase ref after {:?}: {}",
                args,
                line
            );
            assert!(
                !line.starts_with("refs/staircase-state/"),
                "Found unexpected staircase-state ref after {:?}: {}",
                args,
                line
            );
            assert!(
                !line.starts_with("refs/staircase-archive/"),
                "Found unexpected staircase-archive ref after {:?}: {}",
                args,
                line
            );
        }
    }
}

#[test]
fn move_adopts_only_for_stable_or_intermediate_state() {
    // ARRANGE: Setup an implicit staircase with two steps (two branches).
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "feature-1"]);
    context.commit("f1.txt", "f1", "f1 commit");
    let _c1 = context.run_git(&["rev-parse", "HEAD"]);

    context.run_git(&["checkout", "-b", "feature-2"]);
    context.commit("f2a.txt", "f2a", "f2a commit");
    let c2a = context.run_git(&["rev-parse", "HEAD"]);
    context.commit("f2b.txt", "f2b", "f2b commit");
    let _c2b = context.run_git(&["rev-parse", "HEAD"]);

    // Verify it is discovered as implicit with 2 steps.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(stdout.contains("(implicit)"));
    // The name might be "feature-" or "feature-2" depending on discovery.
    // Let's just check for "feature" and "(implicit)".
    assert!(stdout.contains("feature"));

    // ACT: Move a commit between steps.
    // We'll move c2 from step 2 to step 1 (conceptually).
    // Actually, let's just move c2 from feature-2 to feature-1.
    // Wait, move_cmd expects step numbers.
    let (success, stdout, stderr) =
        context.run_staircase(&["move", "--from", "2", "--to", "1", "feature-2", &c2a]);
    if !success {
        panic!("move failed: stdout: {}, stderr: {}", stdout, stderr);
    }

    // ASSERT: Verify it remains implicit if the final layout is discoverable.
    // According to Appendix B, it should remain implicit if the final decomposition is discoverable.
    // Since we moved it and updated branches (presumably), it might be discoverable.
    // But wait, does 'move' update branches?
    // In src/core/manipulation.rs, move calls replay which updates owned branches.

    let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
    let mut adopted = false;
    for line in refs.lines() {
        if line.starts_with("refs/staircases/") {
            adopted = true;
        }
    }

    assert!(
        !adopted,
        "Staircase was adopted but it should have remained implicit as the final decomposition is discoverable via branches."
    );
}

#[test]
fn move_adopts_for_intermediate_state_rewrite() {
    // ARRANGE: Setup an implicit staircase with three steps.
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "f1"]);
    context.commit("f1.txt", "f1", "f1");
    context.run_git(&["checkout", "-b", "f2"]);
    context.commit("f2a.txt", "f2a", "f2a");
    let _c2a = context.run_git(&["rev-parse", "HEAD"]);
    context.commit("f2b.txt", "f2b", "f2b");
    let c2b = context.run_git(&["rev-parse", "HEAD"]);
    context.run_git(&["checkout", "-b", "f3"]);
    context.commit("f3.txt", "f3", "f3");

    // ACT: Move f2b (part of step 2) to AFTER f3 (step 3). This requires a rewrite.
    let (success, stdout, stderr) =
        context.run_staircase(&["move", "--from", "2", "--to", "3", "f3", &c2b]);
    if !success {
        panic!("move failed: stdout: {}, stderr: {}", stdout, stderr);
    }

    // ASSERT: Verify it WAS adopted.
    let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
    let mut adopted = false;
    for line in refs.lines() {
        if line.starts_with("refs/staircases/") {
            adopted = true;
        }
    }
    assert!(
        adopted,
        "Staircase should have been adopted for a move requiring a rewrite (intermediate state)."
    );
}

#[test]
fn reorder_no_restack_does_not_adopt() {
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "f1"]);
    context.commit("f1.txt", "f1", "f1");
    context.run_git(&["checkout", "-b", "f2"]);
    context.commit("f2.txt", "f2", "f2");

    // ACT: Reorder without restack.
    let (success, stdout, stderr) =
        context.run_staircase(&["reorder", "f2", "--steps", "2,1", "--no-restack"]);
    if !success {
        panic!("reorder failed: stdout: {}, stderr: {}", stdout, stderr);
    }

    // ASSERT: Verify it remains implicit.
    let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
    let mut adopted = false;
    for line in refs.lines() {
        if line.starts_with("refs/staircases/") {
            adopted = true;
        }
    }
    assert!(
        !adopted,
        "Staircase should have remained implicit for reorder --no-restack."
    );
}

#[test]
fn drop_no_restack_does_not_adopt() {
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "f1"]);
    context.commit("f1.txt", "f1", "f1");
    context.run_git(&["checkout", "-b", "f2"]);
    context.commit("f2.txt", "f2", "f2");

    // ACT: Drop without restack.
    // wait, drop: selector, step_index (optional).
    // drop f2:1 --leave-descendants-stale
    let (success, stdout, stderr) =
        context.run_staircase(&["drop", "f2:1", "--leave-descendants-stale"]);
    if !success {
        panic!("drop failed: stdout: {}, stderr: {}", stdout, stderr);
    }

    // ASSERT: Verify it remains implicit.
    let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
    let mut adopted = false;
    for line in refs.lines() {
        if line.starts_with("refs/staircases/") {
            adopted = true;
        }
    }
    assert!(
        !adopted,
        "Staircase should have remained implicit for drop --no-restack."
    );
}

#[test]
fn append_adopts_only_for_durable_association() {
    // ARRANGE: Setup an implicit staircase (branch ahead of main).
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "feature"]);
    context.commit("f1.txt", "f1", "f1 commit");

    // Verify it is discovered as implicit.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(stdout.contains("(implicit)"));

    // ACT: Append a commit using git commit.
    context.commit("f2.txt", "f2", "f2 commit");

    // ASSERT: Verify it remains implicit.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(stdout.contains("(implicit)"));
    assert_no_staircase_refs(&context);

    // ACT: Append a commit using staircase append.
    // Create a commit on top of feature, but move the branch back so we can append it.
    context.run_git(&["checkout", "feature"]);
    let c3 = context.commit("f3.txt", "f3", "f3 commit");
    context.run_git(&["reset", "--hard", "HEAD^"]);

    let (success, stdout, stderr) = context.run_staircase(&[
        "append",
        "--commits",
        &format!("feature..{}", c3),
        "feature",
    ]);

    if !success {
        panic!("append failed: stdout: {}, stderr: {}", stdout, stderr);
    }

    // ASSERT: Verify it remains implicit.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(stdout.contains("(implicit)"));
    assert_no_staircase_refs(&context);

    // ACT: Append with a durable association (title).
    // Create another commit to append.
    context.run_git(&["checkout", "feature"]);
    let c4 = context.commit("f4.txt", "f4", "f4 commit");
    context.run_git(&["reset", "--hard", "HEAD^"]);

    let (success, stdout, stderr) = context.run_staircase(&[
        "append",
        "--commits",
        &format!("feature..{}", c4),
        "--title",
        "New Durable Title",
        "feature",
    ]);
    if !success {
        panic!(
            "append with title failed: stdout: {}, stderr: {}",
            stdout, stderr
        );
    }

    // ASSERT: Verify it is now adopted.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(!stdout.contains("(implicit)"));
    assert_has_staircase_refs(&context);
}

fn assert_no_staircase_refs(context: &TestContext) {
    let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
    for line in refs.lines() {
        assert!(
            !line.starts_with("refs/staircases/"),
            "Found unexpected staircase ref: {}",
            line
        );
        assert!(
            !line.starts_with("refs/staircase-state/"),
            "Found unexpected staircase-state ref: {}",
            line
        );
    }
}

fn assert_has_staircase_refs(context: &TestContext) {
    let refs = context.run_git(&["for-each-ref", "--format=%(refname)"]);
    let mut found = false;
    for line in refs.lines() {
        if line.starts_with("refs/staircases/") || line.starts_with("refs/staircase-state/") {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Expected to find staircase refs, but none were found."
    );
}

#[test]
fn persistent_metadata_always_adopts() {
    // ARRANGE: Setup an implicit staircase.
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "feature"]);
    context.commit("f1.txt", "f1", "f1 commit");

    // ACT: Set a title (durable association).
    let (success, stdout, stderr) =
        context.run_staircase(&["metadata", "set-title", "feature", "New Title"]);
    if !success {
        panic!(
            "metadata set-title failed: stdout: {}, stderr: {}",
            stdout, stderr
        );
    }

    // ASSERT: Verify it is now adopted.
    let (success, stdout, _) = context.run_staircase(&["list"]);
    assert!(success);
    assert!(!stdout.contains("(implicit)"));
    assert_has_staircase_refs(&context);
}

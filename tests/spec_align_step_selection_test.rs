mod common;
use common::*;

#[test]
fn test_drop_with_id_and_step() {
    // ARRANGE: Create a managed staircase with a specific lineage ID.
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "step1"]);
    commit(path, "f1.txt", "1", "c1");
    run_git(path, &["checkout", "-b", "step2"]);
    commit(path, "f2.txt", "2", "c2");

    // Adopt it to make it managed and get an ID
    let (success, stdout, stderr) =
        run_staircase(path, &["adopt", "my-staircase", "step1", "step2"]);
    assert!(success, "Adopt failed: {} {}", stdout, stderr);

    // Get the ID
    let (success, id, stderr) = run_staircase(path, &["id", "my-staircase"]);
    assert!(success, "Id failed: {}", stderr);
    let id = id.trim();

    // ACT: Execute git staircase drop --id <id> --step 1
    let (success, stdout, stderr) = run_staircase(path, &["drop", "--id", id, "--step", "1"]);

    // ASSERT: Verify that the first step is removed.
    assert!(
        success,
        "Drop with --id and --step failed: {} {}",
        stdout, stderr
    );

    let (success, steps, stderr) = run_staircase(path, &["steps", "--id", id]);
    assert!(success, "Steps failed: {}", stderr);
    // Should only have step2 now
    assert!(!steps.contains("step1"), "step1 should have been dropped");
    assert!(steps.contains("step2"), "step2 should still exist");
}

#[test]
fn test_split_with_id_and_step() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "step1"]);
    commit(path, "f1.txt", "1", "c1");
    let c2 = commit(path, "f1.txt", "1.1", "c1.1");
    commit(path, "f1.txt", "1.2", "c1.2");

    let (success, _stdout, stderr) = run_staircase(path, &["adopt", "my-staircase", "step1"]);
    assert!(success, "Adopt failed: {}", stderr);

    let (success, id, stderr) = run_staircase(path, &["id", "my-staircase"]);
    assert!(success, "Id failed: {}", stderr);
    let id = id.trim();

    // ACT: git staircase split --id <id> --step 1 --at <c2> --branch step1a
    let (success, stdout, stderr) = run_staircase(
        path,
        &[
            "split", "--id", id, "--step", "1", "--at", &c2, "--branch", "step1a",
        ],
    );
    assert!(
        success,
        "Split with --id and --step failed: {} {}",
        stdout, stderr
    );

    let (success, steps, stderr) = run_staircase(path, &["steps", "--id", id]);
    assert!(success, "Steps failed: {}", stderr);
    assert!(steps.contains("step1a"), "step1a should exist after split");
}

#[test]
fn test_join_with_id_and_steps() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "step1"]);
    commit(path, "f1.txt", "1", "c1");
    run_git(path, &["checkout", "-b", "step2"]);
    commit(path, "f2.txt", "2", "c2");

    let (success, _stdout, stderr) =
        run_staircase(path, &["adopt", "my-staircase", "step1", "step2"]);
    assert!(success, "Adopt failed: {}", stderr);

    let (success, id, stderr) = run_staircase(path, &["id", "my-staircase"]);
    assert!(success, "Id failed: {}", stderr);
    let id = id.trim();

    // ACT: git staircase join --id <id> --step 1 --step2 2
    let (success, stdout, stderr) =
        run_staircase(path, &["join", "--id", id, "--step", "1", "--step2", "2"]);
    assert!(
        success,
        "Join with --id and --step failed: {} {}",
        stdout, stderr
    );

    let (success, steps, stderr) = run_staircase(path, &["steps", "--id", id]);
    assert!(success, "Steps failed: {}", stderr);
    // Should only have one step now (usually the name of the second one is kept or merged)
    assert_eq!(
        steps.lines().count(),
        1,
        "Should have only 1 step after join"
    );
}

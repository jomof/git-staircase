mod common;
use common::TestContext;
use std::fs;

#[test]
fn test_manual_restack_multi_commit_step() {
    let ctx = TestContext::new();

    // ARRANGE: Create a staircase with two steps.
    // Step 1 (s1) has 1 commit.
    // Step 2 (s2) has 2 commits.
    ctx.run_git(&["checkout", "-b", "s1"]);
    ctx.commit("file1.txt", "1", "s1 commit 1");

    ctx.run_git(&["checkout", "-b", "s2"]);
    ctx.commit("file2.txt", "2.1\n", "s2 commit 1");
    ctx.commit("file2.txt", "2.1\n2.2\n", "s2 commit 2");

    // Adopt the staircase
    let (success, stdout, stderr) = ctx.run_staircase(&["adopt", "my-staircase", "s1", "s2"]);
    assert!(
        success,
        "Adopt failed!\nSTDOUT: {}\nSTDERR: {}",
        stdout, stderr
    );

    // ACT: Materialize a change into Step 1.
    // This will trigger a restack of Step 2 onto the new Step 1.
    ctx.run_git(&["checkout", "s1"]);

    // Modify file1.txt. Since it's already tracked, --all-tracked will pick it up.
    let file1_path = ctx.path().join("file1.txt");
    fs::write(file1_path, "1 updated").expect("Failed to write file1.txt");

    let (success, stdout, stderr) =
        ctx.run_staircase(&["draft", "materialize", "--all-tracked", "my-staircase"]);

    // ASSERT: It should succeed.
    assert!(
        success,
        "Materialize failed!\nSTDOUT: {}\nSTDERR: {}",
        stdout, stderr
    );

    // Verify s2 still has two commits above the new s1
    let commits = ctx.run_git(&["rev-list", "s1..s2"]);
    let commit_list: Vec<&str> = commits.lines().collect();

    assert_eq!(
        commit_list.len(),
        2,
        "Step s2 lost commits during restack! Found: {:?}",
        commit_list
    );

    // Verify content of file2.txt in the tip of s2
    let content = ctx.run_git(&["show", "s2:file2.txt"]);
    // ctx.run_git trims output, so we expect "2.1\n2.2"
    assert_eq!(content, "2.1\n2.2", "Content of file2.txt is corrupted!");
}

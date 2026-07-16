use git_staircase::core::*;
use std::fs;

mod common;
use common::*;

#[test]
fn test_manual_restack_silently_creates_conflict_markers() {
    // ARRANGE: Setup a staircase with two steps that conflict
    let ctx = TestContext::new();
    
    // Initial commit
    ctx.commit("file.txt", "line 1\n", "initial");
    
    // Step 1: modifies line 1
    ctx.run_git(&["checkout", "-b", "step1"]);
    ctx.commit("file.txt", "step 1 change\n", "step 1");
    
    // Step 2: also modifies line 1 (descends from step 1)
    ctx.run_git(&["checkout", "-b", "step2"]);
    ctx.commit("file.txt", "step 2 change\n", "step 2");
    
    // Adopt as managed staircase
    ctx.run_git(&["checkout", "main"]);
    let discoveries = discover(&ctx.repo, Some("main"), None, false).unwrap();
    let git_staircase::model::Discovery::Linear(mut s) = discoveries[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "my-staircase".to_string();
    let _managed = adopt(&ctx.repo, &s).unwrap();
    
    // Now, let's create a DRAFT for Step 1 that conflicts with Step 2's change.
    // We'll checkout Step 1 and change the same line.
    ctx.run_git(&["checkout", "step1"]);
    fs::write(ctx.repo.workdir.join("file.txt"), "conflicting step 1 change\n").unwrap();
    
    // ACT: Materialize the draft for Step 1.
    // This should update Step 1 and then RESTACK Step 2.
    // Since it uses Manual strategy, it will use merge-tree.
    let options = MaterializeOptions {
        all_tracked: true,
        ..Default::default()
    };
    let result = materialize_draft(&ctx.repo, Some("my-staircase"), None, &options);
    
    // ASSERT: Should return an error because it's a conflict
    assert!(result.is_err(), "materialize_draft should fail on conflict");
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(err_msg.contains("Merge conflict detected"), "Expected merge conflict error, got: {}", err_msg);
}

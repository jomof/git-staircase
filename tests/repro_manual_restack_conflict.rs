mod common;
use common::*;
use git_staircase::core::{self, MaterializeOptions};
use git_staircase::model::DraftIntent;
use std::fs;

#[test]
fn test_materialize_should_fail_on_conflicts() {
    let context = TestContext::new();

    // ARRANGE: Create a staircase with two steps
    context.commit("file.txt", "line 1\n", "initial");
    context.run_git(&["checkout", "-b", "step1"]);
    context.commit("file.txt", "line 1\nstep 1\n", "step 1");
    context.run_git(&["checkout", "-b", "step2"]);
    context.commit("file.txt", "line 1\nstep 1\nstep 2\n", "step 2");

    // Adopt it
    let discovery = core::discover(&context.repo, Some("main"), None, false).unwrap();
    let mut metadata = discovery
        .into_iter()
        .find_map(|item| match item {
            git_staircase::Discovery::Linear(m) => Some(m),
            _ => None,
        })
        .unwrap();
    metadata.name = "my-staircase".into();
    core::adopt(&context.repo, &metadata).unwrap();

    // Attach to step1
    context.run_git(&["checkout", "step1"]);
    core::attach_draft(
        &context.repo,
        "my-staircase",
        Some("step1"),
        Some(DraftIntent::ExtendStep),
    )
    .unwrap();

    // ACT: Create a conflicting change in step1
    fs::write(
        context.path().join("file.txt"),
        "line 1\nconflicting step 1\n",
    )
    .unwrap();

    // Materialize it.
    let options = MaterializeOptions {
        all_tracked: true,
        ..Default::default()
    };

    // This call SHOULD return an error if there are conflicts during restack.
    // If it succeeds, it's a bug because it silently committed conflict markers.
    let result = core::materialize_draft(&context.repo, None, None, &options);

    // ASSERT: Check that it failed (it currently succeeds, which is the bug)
    assert!(
        result.is_err(),
        "Materialize should FAIL when there are conflicts, but it succeeded!"
    );

    let step2_oid = core::resolve_by_name(&context.repo, "my-staircase")
        .unwrap()
        .metadata()
        .steps[1]
        .cut
        .clone();
    let content = context.run_git(&["cat-file", "-p", &format!("{}:file.txt", step2_oid)]);

    assert!(
        !content.contains("<<<<<<<"),
        "Step 2 should NOT contain conflict markers, but it does!"
    );
}

mod common;
use common::*;
use git_staircase::Discovery;
use git_staircase::core;

#[test]
fn test_canonical_descriptor_format() {
    // ARRANGE: Create and adopt a staircase
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feat-core"]);
    commit(dir, "file1.txt", "1", "commit 1");

    let discovered = core::discover(&repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "my-staircase".to_string();

    let s = core::adopt(&repo, &s).unwrap();

    // ACT: Read the descriptor object
    let ref_name = "refs/staircases/my-staircase";
    let _oid = repo
        .resolve_ref(ref_name)
        .expect("refs/staircases/my-staircase should exist");
    let content = repo.run(&["cat-file", "-p", &format!("{}:structure", ref_name)]).unwrap();

    // ASSERT: Verify it starts with the mandatory header (Spec 8.4)
    assert!(
        content.starts_with("git-staircase-descriptor 1\n"),
        "Descriptor should start with header, but was:\n{}",
        content
    );

    // ASSERT: Verify it follows the KVP format and NOT JSON (Spec 8.5)
    assert!(
        !content.trim().starts_with('{'),
        "Descriptor should not be JSON"
    );
    assert!(content.contains(&format!("lineage {}", s.id)));

    // ASSERT: Verify canonical name is EXCLUDED (Spec 8.5)
    assert!(
        !content.contains("my-staircase"),
        "Descriptor should not contain the staircase name"
    );

    // ACT: Parse the descriptor back using persistence::read_metadata
    let parsed = core::persistence::read_metadata(&repo, "my-staircase")
        .expect("Should be able to parse back");

    // ASSERT: Verify round-tripped object is identical (except for name which is recovered from ref)
    assert_eq!(parsed.id, s.id);
    assert_eq!(parsed.target, s.target);
    assert_eq!(parsed.steps.len(), s.steps.len());
    for (p, original) in parsed.steps.iter().zip(s.steps.iter()) {
        assert_eq!(p.name, original.name);
        assert_eq!(p.cut, original.cut);
    }
}

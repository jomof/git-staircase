
use crate::common::*;
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
    let content = repo
        .run(&["cat-file", "-p", &format!("{}:structure", ref_name)])
        .unwrap();

    // ASSERT: generation-1 structure is canonical JSON.
    let structure: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(structure["schema"], "git-staircase/structure");
    assert_eq!(structure["version"], 1);
    assert_eq!(structure["kind"], "linear");
    assert_eq!(structure["lineage_id"], s.id);
    assert!(structure["policies"].is_object());
    assert!(structure["discovery_overrides"].is_array());

    // ASSERT: Verify canonical name is EXCLUDED (Spec 8.5)
    assert!(
        !content.contains("my-staircase"),
        "Structure should not contain the staircase name"
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

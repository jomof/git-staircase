
use crate::common::*;
use git_staircase::Discovery;
use git_staircase::core;

#[test]
fn test_discover_refs_filter() {
    let ctx = TestContext::new();

    // Create branches in feature/
    ctx.run_git(&["checkout", "-b", "feature/auth-core"]);
    ctx.commit("file1.txt", "1", "commit 1");

    // Create branches in experimental/
    ctx.run_git(&["checkout", "main"]);
    ctx.run_git(&["checkout", "-b", "experimental/test-feat"]);
    ctx.commit("file2.txt", "2", "commit 2");

    // ACT: Run discovery without filter
    let discovered_all = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    assert_eq!(discovered_all.len(), 2);

    // ACT: Run discovery with filter
    let discovered_filtered =
        core::discover(&ctx.repo, Some("main"), Some("refs/heads/feature/*"), false).unwrap();
    assert_eq!(discovered_filtered.len(), 1);
    let Discovery::Linear(ref s) = discovered_filtered[0] else {
        panic!("Expected linear discovery");
    };
    assert_eq!(s.name, "feature/auth-core");
}

#[test]
fn test_discover_families_flag() {
    let ctx = TestContext::new();

    ctx.run_git(&["checkout", "-b", "step1"]);
    ctx.commit("file1.txt", "1", "commit 1");

    ctx.run_git(&["checkout", "-b", "step2a"]);
    ctx.commit("file2a.txt", "2a", "commit 2a");

    ctx.run_git(&["checkout", "step1"]);
    ctx.run_git(&["checkout", "-b", "step2b"]);
    ctx.commit("file2b.txt", "2b", "commit 2b");

    // ACT: Run discovery without families flag (should linearize)
    let discovered_linear = core::discover(&ctx.repo, Some("main"), None, false).unwrap();
    // It should find two paths: (step1, step2a) and (step1, step2b)
    assert_eq!(discovered_linear.len(), 2);
    for d in &discovered_linear {
        assert!(matches!(d, Discovery::Linear(_)));
    }

    // ACT: Run discovery with families flag
    let discovered_family = core::discover(&ctx.repo, Some("main"), None, true).unwrap();
    assert_eq!(discovered_family.len(), 1);
    assert!(matches!(discovered_family[0], Discovery::Ambiguous(_)));
}

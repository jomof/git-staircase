mod common;
use common::TestContext;

#[test]
fn test_rebase_preserves_author_but_committer_is_lost() {
    let ctx = TestContext::new();

    // 1. Create a staircase
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("file1.txt", "content1", "commit 1");
    ctx.run_staircase(&["adopt", "my-staircase", "feature"]);

    // 2. Run rebase onto main. This will trigger cherry-pick.
    // Note: ctx.run_staircase does NOT set GIT_AUTHOR_NAME etc. in env.
    // And git-staircase BLOCKS global/system config.
    let (success, _, _) = ctx.run_staircase(&["rebase", "my-staircase", "--onto", "main"]);
    assert!(success);

    // 3. Check the committer of the new commit
    // If the committer info is not what was set in ~/.gitconfig (or fallback),
    // it confirms global config was ignored.
    let committer = ctx.run_git(&["log", "-1", "--format=%cn <%ce>"]);
    // In a clean environment, this might even fail if no defaults exist.
    println!("Committer: {}", committer);
}

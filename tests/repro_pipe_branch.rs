mod common;
use common::*;

#[test]
fn test_local_branches_with_pipe() -> anyhow::Result<()> {
    // ARRANGE
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    // Create a branch with a pipe in its name
    run_git(repo_path, &["checkout", "-b", "feat|pipe"]);

    // ACT
    let branches = repo.local_branches()?;

    // ASSERT
    let pipe_branch = branches.iter().find(|b| b.refname.contains("feat|pipe"));
    assert!(
        pipe_branch.is_some(),
        "Branch 'feat|pipe' should be found correctly. Found: {:?}",
        branches
    );

    Ok(())
}

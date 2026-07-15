mod common;
use common::*;
use git_staircase::GitRepo;
use git_staircase::core;

#[test]
fn test_resolve_staircase_inferred_develop() {
    let (_tmp, _repo) = setup_repo(); // setup_repo creates main
    // But this test needs develop as default branch.
    // setup_repo is a bit opinionated.
    // I'll just use a fresh TempDir here or manually fix it.
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "develop"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    run_git(&path, &["checkout", "-b", "feat-1"]);
    commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    commit(&path, "feat2.txt", "2", "feat 2");

    let rs = core::resolve_staircase(&repo, "feat", None)
        .unwrap()
        .expect("Should find implicit staircase by inferring develop");
    assert_eq!(rs.metadata().name, "feat");
    assert_eq!(rs.metadata().symbolic_integration_target, "refs/heads/develop");
}

#[test]
fn test_resolve_staircase_with_explicit_onto() {
    let (_tmp, _repo) = setup_repo();
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "develop"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path.clone());

    run_git(&path, &["checkout", "-b", "feat-1"]);
    commit(&path, "feat1.txt", "1", "feat 1");
    run_git(&path, &["checkout", "-b", "feat-2"]);
    commit(&path, "feat2.txt", "2", "feat 2");

    let rs = core::resolve_staircase(&repo, "feat", Some("develop"))
        .unwrap()
        .expect("Should find staircase relative to develop explicitly");
    assert_eq!(rs.metadata().steps.len(), 2);
    assert_eq!(rs.metadata().symbolic_integration_target, "refs/heads/develop");
}

#[test]
fn test_resolve_staircase_inference_upstream() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "base"]);
    commit(path, "base.txt", "base", "base commit");

    run_git(path, &["checkout", "-b", "feat-1"]);
    commit(path, "feat1.txt", "1", "feat 1");

    run_git(path, &["checkout", "-b", "feat-2"]);
    commit(path, "feat2.txt", "2", "feat 2");

    run_git(path, &["branch", "--set-upstream-to=base"]);

    let rs = core::resolve_staircase(&repo, "feat", None)
        .unwrap()
        .expect("Should infer base as boundary via upstream");
    assert_eq!(rs.metadata().symbolic_integration_target, "refs/heads/base");
}

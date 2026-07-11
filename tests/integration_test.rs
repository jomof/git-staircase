mod common;
use common::*;
use git_staircase::core;
use git_staircase::{Discovery, ResolvedStaircase, StaircaseMetadata, Step, VerificationPolicy};
use std::fs;

#[test]
fn test_discover_linear() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    run_git(dir, &["checkout", "-b", "feature/auth-tests"]);
    let c3 = commit(dir, "file3.txt", "3", "commit 3");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    assert_eq!(discovered.len(), 1);
    let Discovery::Linear(ref s) = discovered[0] else {
        panic!("Expected linear discovery");
    };
    assert_eq!(s.name, "feature/auth");
    assert_eq!(s.target, "main");
    assert_eq!(s.steps.len(), 3);

    assert_eq!(s.steps[0].name, "feature/auth-core");
    assert_eq!(s.steps[0].cut, c1);

    assert_eq!(s.steps[1].name, "feature/auth-ui");
    assert_eq!(s.steps[1].cut, c2);

    assert_eq!(s.steps[2].name, "feature/auth-tests");
    assert_eq!(s.steps[2].cut, c3);
}

#[test]
fn test_adopt_and_status() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    assert_eq!(discovered.len(), 1);
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();
    let s = core::adopt(&repo, &s).unwrap();

    let read = core::persistence::read_metadata(&repo, &s.id).unwrap();
    assert_eq!(read.name, "auth");
    assert_eq!(read.steps.len(), 2);

    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(status.is_clean);

    run_git(dir, &["checkout", "feature/auth-ui"]);
    let c2_new = commit(dir, "file2_mod.txt", "2 mod", "commit 2 mod");

    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(!status.is_clean);
    assert_eq!(status.steps[1].actual_oid, Some(c2_new.clone()));

    let found = core::find_by_name(&repo, &s.id).unwrap().unwrap();
    assert_eq!(found.name, "auth");
}

#[test]
fn test_status_stale_and_restack() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let s = core::adopt(&repo, &s).unwrap();

    run_git(dir, &["checkout", "feature/auth-core"]);
    fs::write(dir.join("file1.txt"), "1 amended").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "--amend", "-m", "commit 1 amended"]);
    let c1_amended = repo.resolve_ref("HEAD").unwrap();

    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(!status.is_clean);

    let rs = core::resolve_staircase(&repo, &s.id, None)
        .unwrap()
        .unwrap();
    core::restack(&repo, &rs).unwrap();

    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(status.is_clean);
    assert_eq!(status.steps[0].actual_oid, Some(c1_amended));
    assert_ne!(status.steps[1].actual_oid, Some(c2));
}

#[test]
fn test_split_and_join() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");
    let c1_2 = commit(dir, "file1_2.txt", "1.2", "commit 1.2");
    let c1_3 = commit(dir, "file1_3.txt", "1.3", "commit 1.3");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    let s = core::adopt(&repo, &s).unwrap();

    let rs = core::resolve_staircase(&repo, &s.id, None)
        .unwrap()
        .unwrap();
    core::split(&repo, &rs, 0, &c1_2, Some("feature/auth-core-part1")).unwrap();

    let read = core::persistence::read_metadata(&repo, &s.id).unwrap();
    assert_eq!(read.steps.len(), 2);

    let rs = core::resolve_staircase(&repo, &s.id, None)
        .unwrap()
        .unwrap();
    core::join(&repo, &rs, 0, 1).unwrap();

    let read = core::persistence::read_metadata(&repo, &s.id).unwrap();
    assert_eq!(read.steps.len(), 1);
    assert_eq!(read.steps[0].cut, c1_3);
}

#[test]
fn test_adopt_validation() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let _c2 = commit(dir, "file2.txt", "2", "commit 2");

    let s_empty = StaircaseMetadata {
        id: uuid::Uuid::new_v4().to_string(),
        name: "empty".to_string(),
        target: "main".to_string(),
        verification_policy: None,
        steps: vec![
            Step {
                name: "step1".to_string(),
                cut: c1.clone(),
                branch: Some("feature/auth-core".to_string()),
            },
            Step {
                name: "step2".to_string(),
                cut: c1.clone(),
                branch: Some("feature/auth-core".to_string()),
            },
        ],
    };
    assert!(core::adopt(&repo, &s_empty).is_err());
}

#[test]
fn test_discover_forked() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "step1"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "step2a"]);
    let _c2a = commit(dir, "file2a.txt", "2a", "commit 2a");

    run_git(dir, &["checkout", "step1"]);
    run_git(dir, &["checkout", "-b", "step2b"]);
    let _c2b = commit(dir, "file2b.txt", "2b", "commit 2b");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    assert_eq!(discovered.len(), 1);
    assert!(matches!(discovered[0], Discovery::Ambiguous(_)));
}

#[test]
fn test_verification_aggregate() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let _c2 = commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();
    s.verification_policy = Some(VerificationPolicy {
        build_command: Some("true".to_string()),
        test_command: Some("true".to_string()),
        verify_each_prefix: false,
    });

    core::adopt(&repo, &s).unwrap();

    let results = core::verify(
        &repo,
        &ResolvedStaircase::Managed(s.clone()),
        None,
        None,
        Some(true),
        None,
    )
    .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
}

#[test]
fn test_verification_each_prefix() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let _c2 = commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();
    s.verification_policy = Some(VerificationPolicy {
        build_command: Some("true".to_string()),
        test_command: Some("true".to_string()),
        verify_each_prefix: true,
    });

    core::adopt(&repo, &s).unwrap();

    let results = core::verify(
        &repo,
        &ResolvedStaircase::Managed(s.clone()),
        None,
        None,
        None,
        Some(true),
    )
    .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_split_implicit_staircase() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");
    let c1_2 = commit(dir, "file1_2.txt", "1.2", "commit 1.2");
    let _c1_3 = commit(dir, "file1_3.txt", "1.3", "commit 1.3");

    let discoveries = core::discover(&repo, Some("main")).unwrap();
    let s = match &discoveries[0] {
        Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery"),
    };

    let rs = core::resolve_staircase(&repo, &s.id, None)
        .unwrap()
        .expect("Should find implicit staircase");
    assert!(!rs.is_managed());

    core::split(&repo, &rs, 0, &c1_2, Some("feature/auth-core-part1"))
        .expect("Split should succeed");

    let discoveries_after = core::discover(&repo, Some("main")).unwrap();
    let s_after = match &discoveries_after[0] {
        Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery after split"),
    };

    let rs_after = core::resolve_staircase(&repo, &s_after.id, None)
        .unwrap()
        .expect("Should find staircase after split");
    assert!(!rs_after.is_managed());
    assert_eq!(rs_after.metadata().steps.len(), 2);
}

#[test]
fn test_id_lineage_auto_adopt() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    let discoveries = core::discover(&repo, Some("main")).unwrap();
    let s = match &discoveries[0] {
        Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery"),
    };

    let rs = core::resolve_staircase(&repo, &s.id, None)
        .unwrap()
        .expect("Should find implicit staircase");

    use git_staircase::IdentityKind;
    let id = core::compute_identity(&repo, &rs, IdentityKind::Lineage).unwrap();
    assert!(!id.is_empty());

    let rs_after = core::resolve_staircase(&repo, &id, None)
        .unwrap()
        .expect("Should find staircase");
    assert!(rs_after.is_managed());
    assert_eq!(rs_after.metadata().id, id);
}

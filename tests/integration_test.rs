mod common;
use common::*;
use git_staircase::core;
use git_staircase::{Discovery, StaircaseMetadata, Step, VerificationPolicy};
use std::fs;

#[test]
fn test_discover_linear() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create linear chain: main -> step1 -> step2 -> step3
    // step1: c1
    // step2: c2
    // step3: c3
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
    assert_eq!(s.name, "feature/auth"); // Common prefix "feature/auth"
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
    s.name = "auth".to_string(); // Give it a custom name

    core::adopt(&repo, &s).unwrap();

    // Verify metadata was written
    let read = repo.read_metadata(&s.id).unwrap();
    assert_eq!(read.name, "auth");
    assert_eq!(read.steps.len(), 2);

    // Verify step refs
    assert_eq!(
        repo.resolve_ref(&format!("refs/staircases/{}/steps/feature/auth-core", s.id))
            .unwrap(),
        c1
    );
    assert_eq!(
        repo.resolve_ref(&format!("refs/staircases/{}/steps/feature/auth-ui", s.id))
            .unwrap(),
        c2
    );

    // Verify status is clean
    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(status.is_clean);
    assert_eq!(status.steps[0].actual_oid, Some(c1.clone()));
    assert_eq!(status.steps[1].actual_oid, Some(c2.clone()));
    assert!(!status.steps[0].is_modified);
    assert!(!status.steps[1].is_modified);
    assert!(!status.steps[0].is_stale);
    assert!(!status.steps[1].is_stale);

    // Modify step 2 (commit more to feature/auth-ui)
    run_git(dir, &["checkout", "feature/auth-ui"]);
    let c2_new = commit(dir, "file2_mod.txt", "2 mod", "commit 2 mod");

    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(!status.is_clean); // Not clean because step 2 is modified
    assert_eq!(status.steps[1].actual_oid, Some(c2_new.clone()));
    assert!(status.steps[1].is_modified);
    assert!(!status.steps[1].is_stale); // Still descends from step 1 (c1)

    // Verify we can find by name
    let found = core::find_by_name(&repo, "auth").unwrap().unwrap();
    assert_eq!(found.id, s.id);
}

#[test]
fn test_status_stale_and_restack() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(ref s) = discovered[0] else {
        panic!("Expected linear discovery");
    };
    core::adopt(&repo, s).unwrap();

    // Amend step 1 (feature/auth-core)
    run_git(dir, &["checkout", "feature/auth-core"]);
    // We need to amend. We can use commit --amend.
    // To make a change, we overwrite file1.txt
    fs::write(dir.join("file1.txt"), "1 amended").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "--amend", "-m", "commit 1 amended"]);
    let c1_amended = repo.resolve_ref("HEAD").unwrap();

    assert_ne!(c1, c1_amended);

    // Now status should show step 2 as stale because it descends from c1, not c1_amended.
    // And actual of feature/auth-core is c1_amended.
    // actual of feature/auth-ui is c2.
    // Is c1_amended ancestor of c2? No.
    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(!status.is_clean);
    assert!(status.steps[0].is_modified); // c1_amended != c1
    assert!(!status.steps[0].is_stale);
    assert!(!status.steps[1].is_modified); // c2 == c2
    assert!(status.steps[1].is_stale); // c1_amended is not ancestor of c2

    // Restack
    let rs = core::resolve_staircase(&repo, &s.name, None)
        .unwrap()
        .unwrap();
    core::restack(&repo, &rs).unwrap();

    // Verify it is clean now
    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(status.is_clean);

    let c1_new = status.steps[0].actual_oid.as_ref().unwrap();
    let c2_new = status.steps[1].actual_oid.as_ref().unwrap();

    assert_eq!(c1_new, &c1_amended);
    assert_ne!(c2_new, &c2); // c2 should have been rebased

    // Verify ancestry: c1_new -> c2_new
    assert!(repo.is_ancestor(c1_new, c2_new).unwrap());
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
    let Discovery::Linear(ref s) = discovered[0] else {
        panic!("Expected linear discovery");
    };
    core::adopt(&repo, s).unwrap();

    // Staircase has 1 step: feature/auth-core pointing to c1_3.
    // We want to split it at c1_2.
    // c1_2 is between main (target) and c1_3.
    let rs = core::resolve_staircase(&repo, &s.name, None)
        .unwrap()
        .unwrap();
    core::split(&repo, &rs, 0, &c1_2, Some("feature/auth-core-part1")).unwrap();

    // Verify metadata
    let read = repo.read_metadata(&s.id).unwrap();
    assert_eq!(read.steps.len(), 2);
    assert_eq!(read.steps[0].name, "feature/auth-core-part1");
    assert_eq!(read.steps[0].cut, c1_2);
    assert_eq!(read.steps[1].name, "feature/auth-core");
    assert_eq!(read.steps[1].cut, c1_3);

    // Verify status is clean
    let status = core::get_status(&repo, &s.id).unwrap();
    assert!(status.is_clean);

    // Now join them back
    let rs = core::resolve_staircase(&repo, &s.name, None)
        .unwrap()
        .unwrap();
    core::join(&repo, &rs, 0, 1).unwrap();

    // Verify metadata
    let read = repo.read_metadata(&s.id).unwrap();
    assert_eq!(read.steps.len(), 1);
    assert_eq!(read.steps[0].name, "feature/auth-core");
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

    // Case 1: Identical adjacent cuts (empty step)
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

    // Case 2: Out of order cuts (not ancestry linked)
    run_git(dir, &["checkout", "main"]);
    run_git(dir, &["checkout", "-b", "feature/other"]);
    let c_other = commit(dir, "other.txt", "other", "commit other");

    let s_invalid_order = StaircaseMetadata {
        id: uuid::Uuid::new_v4().to_string(),
        name: "invalid".to_string(),
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
                cut: c_other.clone(),
                branch: Some("feature/other".to_string()),
            },
        ],
    };
    assert!(core::adopt(&repo, &s_invalid_order).is_err());
}

#[test]
fn test_discover_forked() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // Create forked structure:
    // main -> step1
    // step1 -> step2a
    // step1 -> step2b

    run_git(dir, &["checkout", "-b", "step1"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "step2a"]);
    let _c2a = commit(dir, "file2a.txt", "2a", "commit 2a");

    run_git(dir, &["checkout", "step1"]);
    run_git(dir, &["checkout", "-b", "step2b"]);
    let _c2b = commit(dir, "file2b.txt", "2b", "commit 2b");

    let discovered = core::discover(&repo, Some("main")).unwrap();

    // NEW BEHAVIOR: Returns one ambiguous family
    assert_eq!(discovered.len(), 1);

    match &discovered[0] {
        Discovery::Ambiguous(f) => {
            assert_eq!(f.steps.len(), 3); // step1, step2a, step2b
            assert!(f.steps.contains_key("step1"));
            assert!(f.steps.contains_key("step2a"));
            assert!(f.steps.contains_key("step2b"));

            let step1 = &f.steps["step1"];
            assert_eq!(step1.children.len(), 2);
            assert!(step1.children.contains(&"step2a".to_string()));
            assert!(step1.children.contains(&"step2b".to_string()));
        }
        _ => panic!("Expected ambiguous family discovery"),
    }
}

#[test]
fn test_verification_aggregate() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();

    // Set verification policy
    s.verification_policy = Some(VerificationPolicy {
        build_command: Some("true".to_string()),
        test_command: Some("true".to_string()),
        verify_each_prefix: false,
    });

    core::adopt(&repo, &s).unwrap();

    // Verify aggregate
    let results = core::verify(None, &repo, "auth", None, None, Some(true), None).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert_eq!(results[0].step_name, "Aggregate");
    assert_eq!(results[0].cut, c2);

    // Verify evidence recorded
    let ref_name = format!("refs/staircases/{}/verification", s.id);
    assert!(repo.resolve_ref(&ref_name).is_ok());

    // Fail verification
    let results = core::verify(
        None,
        &repo,
        "auth",
        Some("false".to_string()),
        None,
        Some(true),
        None,
    )
    .unwrap();
    assert_eq!(results.len(), 1);
    assert!(!results[0].success);
}

#[test]
fn test_verification_each_prefix() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let c1 = commit(dir, "file1.txt", "1", "commit 1");

    run_git(dir, &["checkout", "-b", "feature/auth-ui"]);
    let c2 = commit(dir, "file2.txt", "2", "commit 2");

    let discovered = core::discover(&repo, Some("main")).unwrap();
    let Discovery::Linear(mut s) = discovered[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "auth".to_string();

    // Set verification policy
    s.verification_policy = Some(VerificationPolicy {
        build_command: Some("true".to_string()),
        test_command: Some("true".to_string()),
        verify_each_prefix: true,
    });

    core::adopt(&repo, &s).unwrap();

    // Verify each prefix
    let results = core::verify(None, &repo, "auth", None, None, None, Some(true)).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].success);
    assert_eq!(results[0].step_name, "feature/auth-core");
    assert_eq!(results[0].cut, c1);
    assert!(results[1].success);
    assert_eq!(results[1].step_name, "feature/auth-ui");
    assert_eq!(results[1].cut, c2);
}

#[test]
fn test_split_implicit_staircase() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");
    let c1_2 = commit(dir, "file1_2.txt", "1.2", "commit 1.2");
    let _c1_3 = commit(dir, "file1_3.txt", "1.3", "commit 1.3");

    let rs = core::resolve_staircase(&repo, "feature/auth-core", None)
        .unwrap()
        .expect("Should find implicit staircase");
    assert!(!rs.is_managed());

    // Attempt to split the implicit staircase
    core::split(&repo, &rs, 0, &c1_2, Some("feature/auth-core-part1"))
        .expect("Split should succeed and preserve implicit status");

    // Verify it is still implicit
    let rs_after = core::resolve_staircase(&repo, "feature/auth-core", None)
        .unwrap()
        .expect("Should find staircase");
    assert!(
        !rs_after.is_managed(),
        "Staircase should have remained implicit"
    );

    let status = core::get_status_metadata(&repo, rs_after.metadata().clone()).unwrap();
    assert_eq!(status.metadata.steps.len(), 2);
    assert_eq!(status.metadata.steps[0].name, "feature/auth-core-part1");
}

#[test]
fn test_id_lineage_auto_adopt() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    run_git(dir, &["checkout", "-b", "feature/auth-core"]);
    let _c1 = commit(dir, "file1.txt", "1", "commit 1");

    let rs = core::resolve_staircase(&repo, "feature/auth-core", None)
        .unwrap()
        .expect("Should find implicit staircase");
    assert!(!rs.is_managed());

    // Request lineage ID
    use git_staircase::IdentityKind;
    let id = core::compute_identity(&repo, &rs, IdentityKind::Lineage).unwrap();
    assert!(!id.is_empty());

    // Verify it is now managed
    let rs_after = core::resolve_staircase(&repo, "feature/auth-core", None)
        .unwrap()
        .expect("Should find staircase");
    assert!(rs_after.is_managed());
    assert_eq!(rs_after.metadata().id, id);
}

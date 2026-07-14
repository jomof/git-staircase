use git_staircase::core::*;
use git_staircase::model::*;
use git_staircase::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

mod common;
use common::*;

// --- repro_id.rs ---

#[test]
fn test_implicit_sub_staircase_id_mismatch() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();
    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .status()
            .unwrap();
        assert!(status.success());
    };

    run_git(&["init", "-b", "main"]);
    fs::write(repo_path.join("file"), "0").unwrap();
    run_git(&["add", "file"]);
    run_git(&["commit", "-m", "initial"]);
    run_git(&["checkout", "-b", "step-1"]);
    fs::write(repo_path.join("file"), "1").unwrap();
    run_git(&["commit", "-am", "step 1"]);
    run_git(&["checkout", "-b", "step-2"]);
    fs::write(repo_path.join("file"), "2").unwrap();
    run_git(&["commit", "-am", "step 2"]);
    let repo = GitRepo::new(repo_path.to_path_buf());
    let step1_oid = repo.resolve_commit("step-1").unwrap();
    let rs1 = resolve_staircase(&repo, &step1_oid, Some("main"))
        .unwrap()
        .unwrap();
    let id1_metadata = rs1.metadata().id.clone();
    println!("ID 1: {}", id1_metadata);

    let step2_oid = repo.resolve_commit("step-2").unwrap();
    let rs2 = resolve_staircase(&repo, &step2_oid, Some("main"))
        .unwrap()
        .unwrap();
    let id2_metadata = rs2.metadata().id.clone();
    println!("ID 2: {}", id2_metadata);

    // The sub-staircase (rs1) should have a different structural ID than the full staircase (rs2)
    assert_ne!(
        id1_metadata, id2_metadata,
        "Metadata ID should be unique to the steps content and updated on truncation"
    );
}

// --- repro_id_inconsistency.rs ---

#[test]
#[ignore]
fn test_id_consistency_when_resolving_prefix() {
    // ARRANGE
    let ctx = TestContext::new();

    // Create two branches: A and B, where B is on top of A
    ctx.run_git(&["checkout", "-b", "branchA"]);
    ctx.commit("a.txt", "a", "A");

    ctx.run_git(&["checkout", "-b", "branchB"]);
    ctx.commit("b.txt", "b", "B");

    // ACT: Resolve by name "branchA". This should give a staircase with only branchA.
    let resolved = resolve_staircase(&ctx.repo, "branchA", Some("main"))
        .unwrap()
        .unwrap();
    let meta = resolved.metadata();

    // ASSERT
    let onto_oid = ctx.repo.resolve_commit("main").unwrap();
    let expected_id =
        git_staircase::core::discovery::compute_implicit_id(&ctx.repo, &onto_oid, &meta.steps)
            .unwrap();

    assert_eq!(
        meta.id, expected_id,
        "In-memory metadata ID should match the structural ID"
    );
}

// --- repro_discovery_cycle.rs ---

#[test]
fn test_discovery_with_duplicate_oids() {
    let (_tmp, repo) = setup_repo();
    let repo_path = &repo.workdir;

    // Create a commit on main
    fs::write(repo_path.join("file_new"), "content").unwrap();
    run_git(repo_path, &["add", "file_new"]);
    run_git(repo_path, &["commit", "-m", "base commit"]);

    // Create two branches at the same OID, ahead of main
    run_git(repo_path, &["checkout", "-b", "branch-a"]);
    fs::write(repo_path.join("file_new"), "modified").unwrap();
    run_git(repo_path, &["add", "file_new"]);
    run_git(repo_path, &["commit", "-m", "work"]);

    run_git(repo_path, &["branch", "branch-b", "branch-a"]);

    let onto = if repo.resolve_ref("master").is_ok() {
        "master"
    } else {
        "main"
    };
    let discoveries = discover(&repo, Some(onto), None, false).unwrap();

    let found = discoveries.iter().any(|d| match d {
        git_staircase::model::Discovery::Linear(m) => m
            .steps
            .iter()
            .any(|s| s.name == "branch-a" || s.name == "branch-b"),
        git_staircase::model::Discovery::Ambiguous(f) => {
            f.steps.contains_key("branch-a") || f.steps.contains_key("branch-b")
        }
    });

    assert!(
        found,
        "Should have discovered branch-a and branch-b even if they point to the same OID"
    );
}

// --- repro_duplicate_step_name.rs ---

#[test]
#[ignore]
fn test_split_duplicate_name() {
    let ctx = TestContext::new();

    // 1. Create a chain: main -> c1 -> c2
    ctx.run_git(&["checkout", "-b", "s1"]);
    let c1 = ctx.commit("1.txt", "1", "c1");
    let c2 = ctx.commit("2.txt", "2", "c2");

    // 2. Adopt as managed staircase with step s1 at c2
    let sc = StaircaseMetadata {
        landing_policy: None,
        id: "test-id".to_string(),
        name: "test".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: "s1-id".to_string(),
            name: "s1".to_string(),
            cut: c2.clone(),
            branch: Some("s1".to_string()),
        }],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    git_staircase::core::adopt(&ctx.repo, &sc).unwrap();
    let rs = ResolvedStaircase::Managed(sc);

    // 3. Split at c1 with name "s1" (which already exists)
    let result = git_staircase::core::manipulation::split(
        &ctx.repo,
        &rs,
        0,
        &c1,
        Some("s1"),
        git_staircase::core::SplitOptions { no_ref: false },
    );

    // FAILURE: Result is Ok(()), meaning a duplicate name was allowed
    assert!(
        result.is_err(),
        "Splitting with an existing step name should fail. Result: {:?}",
        result
    );
}

// --- repro_managed_selection.rs ---

#[test]
fn test_resolve_managed_by_internal_step_branch() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // 1. Create a staircase: A -> B -> C
    run_git(dir, &["checkout", "-b", "branch-a"]);
    let c1 = commit(dir, "a.txt", "a", "a");

    run_git(dir, &["checkout", "-b", "branch-b"]);
    let c2 = commit(dir, "b.txt", "b", "b");

    run_git(dir, &["checkout", "-b", "branch-c"]);
    let c3 = commit(dir, "c.txt", "c", "c");

    // 2. Adopt it to make it managed
    let discoveries = core::discover(&repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(mut s) = discoveries[0].clone() else {
        panic!("Expected linear discovery");
    };
    s.name = "my-staircase".to_string();
    let managed_s = core::adopt(&repo, &s).unwrap();
    let uuid = managed_s.id.clone();

    // 3. Resolve by internal step branch name "branch-b"
    let resolved = core::resolve_staircase(&repo, "branch-b", Some("main"))
        .unwrap()
        .expect("Should resolve");

    // ASSERT: It should resolve to the Managed staircase, not an implicit one,
    // and it should be the full staircase (3 steps), not truncated.
    assert!(resolved.is_managed());
    if let ResolvedStaircase::Managed(meta) = resolved.staircase {
        assert_eq!(meta.id, uuid);
        assert_eq!(meta.name, "my-staircase");
        assert_eq!(meta.steps.len(), 3);
        assert_eq!(meta.steps[0].cut, c1);
        assert_eq!(meta.steps[1].cut, c2);
        assert_eq!(meta.steps[2].cut, c3);
    } else {
        panic!("Expected Managed variant");
    }

    // 4. Resolve by internal step cut OID "c2"
    let resolved_by_oid = core::resolve_staircase(&repo, &c2, Some("main"))
        .unwrap()
        .expect("Should resolve by OID");

    assert!(resolved_by_oid.is_managed());
    assert_eq!(resolved_by_oid.metadata().id, uuid);
}

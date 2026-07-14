mod common;
use common::*;
use git_staircase::cli::{BaseStaircaseSelectorArgs, StaircaseSelectorArgs};
use git_staircase::core;
use git_staircase::model::{StaircaseMetadata, Step};

#[test]
fn test_selector_args_resolve_name() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(path, "feat1.txt", "1", "feat 1");

    let args = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: Some("feat-1".to_string()),
            onto: Some("main".to_string()),
            id: None,
            record: None,
            explicit_name: None,
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };

    let rs = args.resolve(&repo).unwrap();

    assert_eq!(rs.metadata().steps.len(), 1);
    assert_eq!(rs.metadata().steps[0].cut, c1);
}

#[test]
fn test_selector_args_resolve_explicit_steps() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(path, "feat1.txt", "1", "feat 1");
    run_git(path, &["checkout", "-b", "feat-2"]);
    let c2 = commit(path, "feat2.txt", "2", "feat 2");

    let args = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: None,
            onto: Some("main".to_string()),
            id: None,
            record: None,
            explicit_name: None,
            r#ref: None,
            structural_key: None,
        },
        steps: Some(vec!["feat-1".to_string(), "feat-2".to_string()]),
    };

    let rs = args.resolve(&repo).unwrap();

    assert_eq!(rs.metadata().steps.len(), 2);
    assert_eq!(rs.metadata().steps[0].cut, c1);
    assert_eq!(rs.metadata().steps[1].cut, c2);
}

#[test]
fn test_selector_args_resolve_id() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(path, "feat1.txt", "1", "feat 1");

    let sc = StaircaseMetadata {
        landing_policy: None,
        id: "my-id".to_string(),
        name: "my-sc".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "feat-1".to_string(),
            cut: c1,
            branch: Some("feat-1".to_string()),
        }],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    core::adopt(&repo, &sc).unwrap();

    let args = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: None,
            onto: None,
            id: Some("my-id".to_string()),
            record: None,
            explicit_name: None,
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };

    let rs = args.resolve(&repo).unwrap();
    assert!(rs.is_managed());
    assert_eq!(rs.metadata().id, "my-id");
}

#[test]
fn test_selector_args_resolve_explicit_name() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(path, "feat1.txt", "1", "feat 1");

    let sc = StaircaseMetadata {
        landing_policy: None,
        id: "my-id".to_string(),
        name: "my-sc".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "feat-1".to_string(),
            cut: c1,
            branch: Some("feat-1".to_string()),
        }],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    core::adopt(&repo, &sc).unwrap();

    let args = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: None,
            onto: None,
            id: None,
            record: None,
            explicit_name: Some("my-sc".to_string()),
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };

    let rs = args.resolve(&repo).unwrap();
    assert!(rs.is_managed());
    assert_eq!(rs.metadata().name, "my-sc");
}

#[test]
fn test_selector_args_ambiguity() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    // Create a conflict between a branch name and a managed staircase name
    run_git(path, &["checkout", "-b", "conflict"]);
    let c1 = commit(path, "f1.txt", "1", "c1");

    let sc = StaircaseMetadata {
        landing_policy: None,
        id: "my-id".to_string(),
        name: "my-sc".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "conflict".to_string(),
            cut: c1,
            branch: Some("conflict".to_string()),
        }],
        verification_policy: None,

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    core::adopt(&repo, &sc).unwrap();

    run_git(path, &["checkout", "main"]);
    run_git(path, &["checkout", "-b", "my-sc"]);
    let _c2 = commit(path, "f2.txt", "2", "c2");

    let args = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: Some("my-sc".to_string()),
            onto: Some("main".to_string()),
            id: None,
            record: None,
            explicit_name: None,
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };

    let result = args.resolve(&repo);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("ambiguous"));
}

#[test]
fn test_core_list_managed() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let c1 = commit(path, "feat1.txt", "1", "feat 1");

    let sc = StaircaseMetadata {
        landing_policy: None,
        id: "my-id".to_string(),
        name: "my-sc".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: String::new(),
            name: "feat-1".to_string(),
            cut: c1.clone(),
            branch: Some("feat-1".to_string()),
        }],
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    core::adopt(&repo, &sc).unwrap();

    let filter = core::ListFilter {
        managed: true,
        ..Default::default()
    };
    let results = core::list(&repo, filter).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].is_managed());
    assert_eq!(results[0].metadata().id, "my-id");
}

#[test]
fn test_core_list_implicit() {
    let (_tmp, repo) = setup_repo();
    let path = &repo.workdir;

    run_git(path, &["checkout", "-b", "feat-1"]);
    let _c1 = commit(path, "feat1.txt", "1", "feat 1");

    let filter = core::ListFilter {
        implicit: true,
        ..Default::default()
    };
    let results = core::list(&repo, filter).unwrap();
    assert_eq!(results.len(), 1);
    assert!(!results[0].is_managed());
}

mod common;

use common::*;
use git_staircase::core;
use git_staircase::error::StaircaseError;
use git_staircase::model::Discovery;
use std::process::Command;

fn command_output(path: &std::path::Path, args: &[&str]) -> std::process::Output {
    let ws_dir = std::env::temp_dir().join(format!(".ws_storage_{:p}", path));
    let bin_str = env!("CARGO_BIN_EXE_git-staircase");
    let mut binary = std::path::PathBuf::from(bin_str);
    if bin_str.contains("/shadow-") || !binary.exists() {
        let fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join("git-staircase");
        if fallback.exists() {
            binary = fallback;
        }
    }
    match Command::new(&binary)
        .current_dir(path)
        .env("GIT_STAIRCASE_WORKSPACE_DIR", &ws_dir)
        .args(args)
        .output()
    {
        Ok(out) => out,
        Err(e) => panic!(
            "Failed to run binary '{:?}' in dir '{:?}': {}",
            binary, path, e
        ),
    }
}

fn adopt_one(repo: &git_staircase::GitRepo, name: &str) -> git_staircase::StaircaseMetadata {
    let discoveries = core::discover(repo, Some("main"), None, false).unwrap();
    let Discovery::Linear(mut metadata) = discoveries[0].clone() else {
        panic!("expected linear staircase");
    };
    metadata.name = name.to_string();
    core::adopt(repo, &metadata).unwrap()
}

#[test]
fn empty_list_output_contracts_are_exact() {
    let (tmp, _) = setup_repo();

    let human = command_output(tmp.path(), &["list"]);
    assert!(human.status.success());
    assert_eq!(human.stdout, b"No staircases.\n");

    let porcelain = command_output(tmp.path(), &["list", "--porcelain"]);
    assert!(porcelain.status.success());
    assert!(porcelain.stdout.is_empty());

    let json = command_output(tmp.path(), &["list", "--json"]);
    assert!(json.status.success());
    assert_eq!(json.stdout, b"[]\n");
    assert!(
        serde_json::from_slice::<serde_json::Value>(&json.stdout)
            .unwrap()
            .is_array()
    );
}

#[test]
fn duplicate_evidence_collapses_to_one_canonical_candidate() {
    let (_tmp, repo) = setup_repo();
    run_git(&repo.workdir, &["checkout", "-b", "zeta"]);
    let cut = commit(&repo.workdir, "feature.txt", "feature", "feature");
    run_git(&repo.workdir, &["branch", "alpha", &cut]);

    let discoveries = core::discover(&repo, Some("main"), None, false).unwrap();
    let linear: Vec<_> = discoveries
        .into_iter()
        .filter_map(|item| match item {
            Discovery::Linear(metadata) => Some(metadata),
            Discovery::Ambiguous(_) => None,
        })
        .collect();

    assert_eq!(linear.len(), 1);
    assert_eq!(linear[0].name, "alpha");
    assert_eq!(linear[0].steps[0].cut, cut);
}

#[test]
fn structural_key_is_full_stable_sha256_and_ignores_names() {
    let (_tmp, repo) = setup_repo();
    run_git(&repo.workdir, &["checkout", "-b", "one"]);
    let cut = commit(&repo.workdir, "feature.txt", "feature", "feature");
    let integration = repo.resolve_commit("main").unwrap();
    let mut left = git_staircase::Step {
        id: String::new(),
        name: "one".into(),
        cut,
        branch: Some("one".into()),
    };
    let first =
        core::discovery::compute_implicit_id(&repo, &integration, std::slice::from_ref(&left))
            .unwrap();
    left.name = "renamed".into();
    left.branch = Some("renamed".into());
    let second = core::discovery::compute_implicit_id(&repo, &integration, &[left]).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.len(), "implicit@".len() + 64);
    assert!(
        first["implicit@".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
}

#[test]
fn stale_full_record_compare_and_swap_is_rejected() {
    let (_tmp, repo) = setup_repo();
    run_git(&repo.workdir, &["checkout", "-b", "feature"]);
    commit(&repo.workdir, "feature.txt", "feature", "feature");
    let adopted = adopt_one(&repo, "managed");
    let record_ref = core::refs::StaircaseRefs::state_record(&adopted.id);
    let old = core::read_record(&repo, &record_ref).unwrap();

    let mut metadata = old.metadata.clone();
    metadata.name = adopted.name.clone();
    let mut changed_user_metadata = old.user_metadata.clone();
    changed_user_metadata.title = Some("first writer".into());
    let current = core::write_record(
        &repo,
        &metadata,
        &changed_user_metadata,
        &old.lifecycle,
        old.archive_manifest.as_ref(),
        Some(&old.record_oid),
        true,
    )
    .unwrap();

    let error = core::write_record(
        &repo,
        &metadata,
        &old.user_metadata,
        &old.lifecycle,
        old.archive_manifest.as_ref(),
        Some(&old.record_oid),
        true,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StaircaseError::ConcurrentRecordUpdate { .. }
    ));
    assert_eq!(error.code(), "concurrent-record-update");
    assert_eq!(repo.resolve_ref(&record_ref).unwrap(), current.record_oid);
}

#[test]
fn public_and_internal_record_refs_move_together() {
    let (_tmp, repo) = setup_repo();
    run_git(&repo.workdir, &["checkout", "-b", "feature"]);
    commit(&repo.workdir, "feature.txt", "feature", "feature");
    let adopted = adopt_one(&repo, "managed");
    let public = core::refs::StaircaseRefs::public("managed");
    let internal = core::refs::StaircaseRefs::state_record(&adopted.id);
    assert_eq!(
        repo.resolve_ref(&public).unwrap(),
        repo.resolve_ref(&internal).unwrap()
    );

    let old = core::read_record(&repo, &internal).unwrap();
    let mut metadata = old.metadata.clone();
    metadata.name = adopted.name;
    let mut user_metadata = old.user_metadata.clone();
    user_metadata.description = Some("updated".into());
    let updated = core::write_record(
        &repo,
        &metadata,
        &user_metadata,
        &old.lifecycle,
        None,
        Some(&old.record_oid),
        true,
    )
    .unwrap();

    assert_eq!(repo.resolve_ref(&public).unwrap(), updated.record_oid);
    assert_eq!(repo.resolve_ref(&internal).unwrap(), updated.record_oid);
}

#[test]
fn ambiguous_selector_has_typed_machine_diagnostics() {
    let (tmp, repo) = setup_repo();
    run_git(tmp.path(), &["checkout", "-b", "managed-source"]);
    commit(tmp.path(), "managed.txt", "managed", "managed");
    adopt_one(&repo, "collision");
    run_git(tmp.path(), &["checkout", "main"]);
    run_git(tmp.path(), &["checkout", "-b", "collision"]);
    commit(tmp.path(), "implicit.txt", "implicit", "implicit");

    let output = command_output(
        tmp.path(),
        &["show", "collision", "--onto", "main", "--json"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["error"]["code"], "selector-ambiguous");
    assert!(
        diagnostic["error"]["details"]["candidates"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
}

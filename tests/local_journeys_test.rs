mod common;

use common::*;
use git_staircase::Step;
use git_staircase::core::{self, DraftRecovery, OperationJournal, OperationPhase};
use git_staircase::model::StaircaseMetadata;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use std::process::Command;

fn adopt_feature(context: &TestContext, name: &str) -> StaircaseMetadata {
    context.run_git(&["checkout", "-b", "feature"]);
    context.commit("feature.txt", "feature\n", "feature");
    let discovery = core::discover(&context.repo, Some("main"), None, false).unwrap();
    let mut metadata = discovery
        .into_iter()
        .find_map(|item| match item {
            git_staircase::Discovery::Linear(metadata) => Some(metadata),
            _ => None,
        })
        .unwrap();
    metadata.name = name.into();
    core::adopt(&context.repo, &metadata).unwrap()
}

fn write_journal(context: &TestContext, journal: &OperationJournal) {
    let directory = context
        .repo
        .common_dir()
        .unwrap()
        .join("staircase")
        .join("journals");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join(format!("{}.json", journal.operation_id)),
        serde_json::to_vec_pretty(journal).unwrap(),
    )
    .unwrap();
}

fn get_binary_path() -> std::path::PathBuf {
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
    binary
}

#[test]
fn canonical_local_command_surface_is_exposed() {
    let context = TestContext::new();
    let output = Command::new(get_binary_path())
        .current_dir(context.path())
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for command in [
        "append",
        "normalize",
        "discovery",
        "policy",
        "layout",
        "continue",
        "abort",
        "operation",
        "tag",
        "rev-parse",
        "push",
        "fetch",
        "name",
        "rename",
        "unname",
    ] {
        assert!(help.contains(command), "missing command {command}");
    }
}

#[test]
fn policy_and_discovery_are_structural_record_revisions() {
    let context = TestContext::new();
    let metadata = adopt_feature(&context, "managed");
    let record_ref = core::refs::StaircaseRefs::state_record(&metadata.id);
    let before = core::read_record(&context.repo, &record_ref).unwrap();

    let policy = context.run_staircase(&[
        "policy",
        "set",
        "managed",
        "merge.mode=\"forbid\"",
        "--json",
    ]);
    assert!(policy.0, "{}", policy.2);
    let after_policy = core::read_record(&context.repo, &record_ref).unwrap();
    assert_ne!(after_policy.record_oid, before.record_oid);
    assert_ne!(after_policy.structure_oid, before.structure_oid);
    assert_eq!(after_policy.metadata_oid, before.metadata_oid);
    let structure = context.run_git(&["cat-file", "-p", &after_policy.structure_oid]);
    let structure: serde_json::Value = serde_json::from_str(&structure).unwrap();
    assert_eq!(structure["policies"]["merge.mode"], "forbid");

    let discovery = context.run_staircase(&[
        "discovery",
        "include-ref",
        "managed",
        "refs/heads/feature",
        "--json",
    ]);
    assert!(discovery.0, "{}", discovery.2);
    let after_discovery = core::read_record(&context.repo, &record_ref).unwrap();
    assert_ne!(after_discovery.structure_oid, after_policy.structure_oid);
    assert_eq!(after_discovery.metadata_oid, after_policy.metadata_oid);
    let structure = context.run_git(&["cat-file", "-p", &after_discovery.structure_oid]);
    let structure: serde_json::Value = serde_json::from_str(&structure).unwrap();
    assert_eq!(structure["discovery_overrides"][0]["kind"], "include-ref");
}

#[test]
fn annotated_snapshot_tag_supports_configured_openpgp_signer() {
    let context = TestContext::new();
    adopt_feature(&context, "managed");
    let signer = context.path().join("fake-gpg.sh");
    fs::write(
        &signer,
        "#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do shift; done\n\
         printf '%s\\n' '-----BEGIN PGP SIGNATURE-----' '' 'ZmFrZQ==' \
         '-----END PGP SIGNATURE-----'\n",
    )
    .unwrap();
    fs::set_permissions(&signer, fs::Permissions::from_mode(0o755)).unwrap();
    context.run_git(&["config", "gpg.format", "openpgp"]);
    context.run_git(&["config", "gpg.program", signer.to_str().unwrap()]);
    context.run_git(&["config", "user.signingkey", "test-key"]);

    let tagged = context.run_staircase(&[
        "tag",
        "signed-snapshot",
        "managed",
        "--message",
        "signed snapshot",
        "--sign",
        "--json",
    ]);
    assert!(tagged.0, "{}", tagged.2);
    let tag = context.run_git(&["cat-file", "-p", "refs/tags/staircase/signed-snapshot"]);
    assert!(tag.contains("type tree"));
    assert!(tag.contains("signed snapshot"));
    assert!(tag.contains("-----BEGIN PGP SIGNATURE-----"));
}

#[test]
fn conflict_pause_has_recovery_refs_and_continue_is_deterministic() {
    let context = TestContext::new();
    fs::write(context.path().join("conflict.txt"), "base\n").unwrap();
    context.run_git(&["add", "conflict.txt"]);
    context.run_git(&["commit", "-m", "conflict base"]);
    context.run_git(&["checkout", "-b", "feature"]);
    fs::write(context.path().join("conflict.txt"), "feature\n").unwrap();
    context.run_git(&["commit", "-am", "feature conflict"]);
    let metadata = {
        let discovery = core::discover(&context.repo, Some("main"), None, false).unwrap();
        let mut metadata = discovery
            .into_iter()
            .find_map(|item| match item {
                git_staircase::Discovery::Linear(metadata) => Some(metadata),
                _ => None,
            })
            .unwrap();
        metadata.name = "managed".into();
        core::adopt(&context.repo, &metadata).unwrap()
    };
    let step_id = metadata.steps[0].id.clone();
    context.run_git(&["checkout", "main"]);
    fs::write(context.path().join("conflict.txt"), "main\n").unwrap();
    context.run_git(&["commit", "-am", "main conflict"]);

    let paused = context.run_staircase(&["rebase", "managed", "--onto", "main", "--json"]);
    assert!(!paused.0);
    assert!(paused.2.contains("operation-paused"), "{}", paused.2);
    let active = core::active_operation(&context.repo).unwrap().unwrap();
    assert_eq!(active.phase, OperationPhase::Paused);
    assert!(active.continuation.is_some());
    assert_eq!(
        core::external_git_operation(&context.repo).unwrap(),
        Some(("cherry-pick".into(), "git staircase continue|abort".into()))
    );
    let recovery_prefix = format!("refs/staircase-recovery/{}/", active.operation_id);
    assert!(
        !context
            .run_git(&["for-each-ref", "--format=%(refname)", &recovery_prefix])
            .is_empty()
    );

    fs::write(context.path().join("conflict.txt"), "resolved\n").unwrap();
    context.run_git(&["add", "conflict.txt"]);
    let continued = context.run_staircase(&["continue", "--json"]);
    assert!(continued.0, "{}", continued.2);
    assert!(core::active_operation(&context.repo).unwrap().is_none());
    assert!(
        context
            .run_git(&["for-each-ref", "--format=%(refname)", &recovery_prefix])
            .is_empty()
    );
    let updated = core::persistence::read_metadata(&context.repo, "managed").unwrap();
    assert_eq!(updated.steps[0].id, step_id);
    assert_eq!(
        context.run_git(&["show", "feature:conflict.txt"]),
        "resolved"
    );
}

#[test]
fn conflict_abort_restores_index_worktree_and_deleted_untracked_bytes() {
    let context = TestContext::new();
    fs::write(context.path().join("conflict.txt"), "base\n").unwrap();
    context.run_git(&["add", "conflict.txt"]);
    context.run_git(&["commit", "-m", "conflict base"]);
    context.run_git(&["checkout", "-b", "feature"]);
    fs::write(context.path().join("conflict.txt"), "feature\n").unwrap();
    context.run_git(&["commit", "-am", "feature conflict"]);
    let adoption = context.run_staircase(&["adopt", "managed", "feature"]);
    assert!(adoption.0, "{}", adoption.2);
    context.run_git(&["checkout", "main"]);
    fs::write(context.path().join("conflict.txt"), "main\n").unwrap();
    context.run_git(&["commit", "-am", "main conflict"]);

    fs::write(context.path().join("draft.txt"), "staged\n").unwrap();
    context.run_git(&["add", "draft.txt"]);
    let index_tree = context.run_git(&["write-tree"]);
    fs::write(context.path().join("draft.txt"), "unstaged\n").unwrap();
    let binary = vec![0, 9, 0xff, 3, 0];
    fs::write(context.path().join("untracked.bin"), &binary).unwrap();
    symlink("untracked.bin", context.path().join("untracked-link")).unwrap();

    let paused = context.run_staircase(&["rebase", "managed", "--onto", "main"]);
    assert!(!paused.0);
    assert!(paused.2.contains("operation-paused"), "{}", paused.2);
    fs::remove_file(context.path().join("untracked.bin")).unwrap();
    fs::remove_file(context.path().join("untracked-link")).unwrap();
    let aborted = context.run_staircase(&["abort", "--json"]);
    assert!(aborted.0, "{}", aborted.2);
    assert_eq!(context.run_git(&["write-tree"]), index_tree);
    assert_eq!(
        fs::read_to_string(context.path().join("draft.txt")).unwrap(),
        "unstaged\n"
    );
    assert_eq!(
        fs::read(context.path().join("untracked.bin")).unwrap(),
        binary
    );
    assert_eq!(
        fs::read_link(context.path().join("untracked-link")).unwrap(),
        std::path::PathBuf::from("untracked.bin")
    );
}

#[test]
fn complex_move_and_partial_restack_preserve_step_ids() {
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "s1"]);
    let _a = context.commit("a.txt", "a\n", "a");
    let b = context.commit("b.txt", "b\n", "b");
    context.run_git(&["checkout", "-b", "s2"]);
    context.commit("c.txt", "c\n", "c");
    let adopted = context.run_staircase(&["adopt", "managed", "s1", "s2"]);
    assert!(adopted.0, "{}", adopted.2);
    let before = core::persistence::read_metadata(&context.repo, "managed").unwrap();
    let ids = before
        .steps
        .iter()
        .map(|step| step.id.clone())
        .collect::<Vec<_>>();

    let moved = context.run_staircase(&["move", "managed", "--from", "1", "--to", "2", &b]);
    assert!(moved.0, "{}", moved.2);
    let after_move = core::persistence::read_metadata(&context.repo, "managed").unwrap();
    assert_eq!(
        after_move
            .steps
            .iter()
            .map(|step| step.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        context.run_git(&["log", "--format=%s", "--reverse", "main..s2"]),
        "a\nc\nb"
    );

    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "p1"]);
    context.commit("p1.txt", "p1\n", "p1");
    context.run_git(&["checkout", "-b", "p2"]);
    context.commit("p2.txt", "p2\n", "p2");
    let adopted = context.run_staircase(&["adopt", "partial", "p1", "p2"]);
    assert!(adopted.0, "{}", adopted.2);
    let partial_before = core::persistence::read_metadata(&context.repo, "partial").unwrap();
    let ids = partial_before
        .steps
        .iter()
        .map(|step| step.id.clone())
        .collect::<Vec<_>>();
    context.commit("d.txt", "d\n", "d");
    let restacked = context.run_staircase(&["restack", "partial", "--from", "partial:2"]);
    assert!(restacked.0, "{}", restacked.2);
    let after_restack = core::persistence::read_metadata(&context.repo, "partial").unwrap();
    assert_eq!(
        after_restack
            .steps
            .iter()
            .map(|step| step.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
}

#[test]
fn partial_landing_keeps_surviving_ids_and_renumbers_layout() {
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "s1"]);
    let landed = context.commit("one.txt", "one\n", "one");
    context.run_git(&["checkout", "-b", "s2"]);
    context.commit("two.txt", "two\n", "two");
    context.run_git(&["checkout", "-b", "s3"]);
    context.commit("three.txt", "three\n", "three");
    let adopted = context.run_staircase(&["adopt", "managed", "s1", "s2", "s3"]);
    assert!(adopted.0, "{}", adopted.2);
    context.run_git(&["checkout", "main"]);
    let layout = context.run_staircase(&["layout", "set", "managed", "--base", "managed"]);
    assert!(layout.0, "{}", layout.2);
    let before = core::persistence::read_metadata(&context.repo, "managed").unwrap();
    let old_main = context.run_git(&["rev-parse", "main"]);
    let surviving_ids = before.steps[1..]
        .iter()
        .map(|step| step.id.clone())
        .collect::<Vec<_>>();

    let preview =
        context.run_staircase(&["land", "managed", "--through", "managed:1", "--dry-run"]);
    assert!(preview.0, "{}", preview.2);
    assert_eq!(context.run_git(&["rev-parse", "main"]), old_main);
    let landed_result = context.run_staircase(&["land", "managed", "--through", "managed:1"]);
    assert!(landed_result.0, "{}", landed_result.2);
    assert_eq!(context.run_git(&["rev-parse", "main"]), landed);
    let after = core::persistence::read_metadata(&context.repo, "managed").unwrap();
    assert_eq!(
        after
            .steps
            .iter()
            .map(|step| step.id.clone())
            .collect::<Vec<_>>(),
        surviving_ids
    );
    assert_eq!(
        after
            .steps
            .iter()
            .map(|step| step.branch.clone().unwrap())
            .collect::<Vec<_>>(),
        vec!["managed-1", "managed"]
    );
    let record = core::read_record(
        &context.repo,
        &core::refs::StaircaseRefs::state_record(&after.id),
    )
    .unwrap();
    let structure = context.run_git(&["cat-file", "-p", &record.structure_oid]);
    let structure: serde_json::Value = serde_json::from_str(&structure).unwrap();
    assert_eq!(structure["structural_state"]["kind"], "partially-landed");
}

#[test]
fn snapshot_restores_deleted_binary_untracked_and_symlink_losslessly() {
    let context = TestContext::new();
    fs::write(context.path().join("tracked.txt"), "staged\n").unwrap();
    context.run_git(&["add", "tracked.txt"]);
    let index_tree = context.run_git(&["write-tree"]);
    fs::write(context.path().join("tracked.txt"), "unstaged\n").unwrap();
    let binary = vec![0, 1, 2, 0xff, 0x7f];
    fs::write(context.path().join("binary.dat"), &binary).unwrap();
    symlink("binary.dat", context.path().join("binary-link")).unwrap();

    let snapshot = context.run_staircase(&["draft", "snapshot", "--json"]);
    assert!(snapshot.0, "{}", snapshot.2);
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot.1).unwrap();
    let id = snapshot["id"].as_str().unwrap();
    context.run_git(&["reset", "--hard", "HEAD"]);
    fs::remove_file(context.path().join("binary.dat")).unwrap();
    fs::remove_file(context.path().join("binary-link")).unwrap();

    let restored = context.run_staircase(&["draft", "restore", id]);
    assert!(restored.0, "{}", restored.2);
    assert_eq!(context.run_git(&["write-tree"]), index_tree);
    assert_eq!(
        fs::read_to_string(context.path().join("tracked.txt")).unwrap(),
        "unstaged\n"
    );
    assert_eq!(fs::read(context.path().join("binary.dat")).unwrap(), binary);
    assert_eq!(
        fs::read_link(context.path().join("binary-link")).unwrap(),
        std::path::PathBuf::from("binary.dat")
    );
}

#[test]
fn active_operation_blocks_mutation_and_abort_restores_leased_ref() {
    let context = TestContext::new();
    let old = context.run_git(&["rev-parse", "HEAD"]);
    let new = context.commit("next.txt", "next\n", "next");
    context.run_git(&["update-ref", "refs/heads/recovery-target", &new]);
    let reference = "refs/heads/recovery-target".to_string();
    let journal = OperationJournal {
        schema: "git-staircase/operation-journal".into(),
        version: 1,
        operation_id: "00000000-0000-4000-8000-000000000001".into(),
        kind: "rebase".into(),
        phase: OperationPhase::Paused,
        repository_identity: context.repo.repository_identity().unwrap(),
        lineage_id: None,
        expected_record_revision: None,
        expected_refs: BTreeMap::from([(reference.clone(), Some(old.clone()))]),
        planned_refs: BTreeMap::from([(reference.clone(), Some(new))]),
        recovery_refs: BTreeMap::new(),
        draft: None,
        disposition: "continue-or-abort".into(),
        continuation: None,
    };
    write_journal(&context, &journal);

    let blocked = context.run_staircase(&["fetch", "--dry-run", "--json"]);
    assert!(!blocked.0);
    assert!(blocked.2.contains("operation-in-progress"));

    let shown = context.run_staircase(&["operation", "show", "--json"]);
    assert!(shown.0, "{}", shown.2);
    assert!(shown.1.contains(&journal.operation_id));

    let aborted = context.run_staircase(&["abort", "--json"]);
    assert!(aborted.0, "{}", aborted.2);
    assert_eq!(
        context.run_git(&["rev-parse", "refs/heads/recovery-target"]),
        old
    );
    assert!(core::active_operation(&context.repo).unwrap().is_none());
}

#[test]
fn abort_restores_exact_index_and_unstaged_worktree_content() {
    let context = TestContext::new();
    fs::write(context.path().join("draft.txt"), "staged\n").unwrap();
    context.run_git(&["add", "draft.txt"]);
    let index_tree = context.run_git(&["write-tree"]);
    fs::write(context.path().join("draft.txt"), "unstaged\n").unwrap();
    let patch = context
        .repo
        .command()
        .args(["diff", "--binary", "--no-ext-diff"])
        .trim(false)
        .run()
        .unwrap();
    let head = context.run_git(&["rev-parse", "HEAD"]);
    let journal = OperationJournal {
        schema: "git-staircase/operation-journal".into(),
        version: 1,
        operation_id: "00000000-0000-4000-8000-000000000002".into(),
        kind: "restack".into(),
        phase: OperationPhase::Paused,
        repository_identity: context.repo.repository_identity().unwrap(),
        lineage_id: None,
        expected_record_revision: None,
        expected_refs: BTreeMap::new(),
        planned_refs: BTreeMap::new(),
        recovery_refs: BTreeMap::new(),
        draft: Some(DraftRecovery {
            head_oid: head,
            head_ref: Some("refs/heads/main".into()),
            index_tree_oid: Some(index_tree.clone()),
            index_snapshot: None,
            dirty_files: Vec::new(),
            unstaged_patch: patch,
            untracked_paths: Vec::new(),
            untracked_files: Vec::new(),
            attachment_json: None,
        }),
        disposition: "continue-or-abort".into(),
        continuation: None,
    };
    write_journal(&context, &journal);
    context.run_git(&["reset", "--hard", "HEAD"]);

    let aborted = context.run_staircase(&["abort"]);
    assert!(aborted.0, "{}", aborted.2);
    assert_eq!(context.run_git(&["write-tree"]), index_tree);
    assert_eq!(
        fs::read_to_string(context.path().join("draft.txt")).unwrap(),
        "unstaged\n"
    );
}

#[test]
fn metadata_editor_rejects_concurrent_full_record_change() {
    let context = TestContext::new();
    adopt_feature(&context, "managed");
    let editor = context.path().join("concurrent-editor.sh");
    let bin_path = get_binary_path();
    fs::write(
        &editor,
        format!(
            "#!/bin/sh\n'{}' archive managed --snapshot-drafts >/dev/null\n",
            bin_path.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(&bin_path)
        .current_dir(context.path())
        .env("GIT_EDITOR", &editor)
        .args(["metadata", "edit", "managed", "--json"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("concurrent-record-update"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let archived = core::list_archived_staircases(&context.repo).unwrap();
    assert_eq!(archived.len(), 1);
    assert!(
        archived[0]
            .user_metadata
            .as_ref()
            .and_then(|metadata| metadata.title.as_ref())
            .is_none()
    );
}

#[test]
fn staircase_transport_uses_distinct_explicit_namespaces() {
    let context = TestContext::new();
    let bare = context.path().join("remote.git");
    context.run_git(&["init", "--bare", bare.to_str().unwrap()]);
    context.run_git(&["remote", "add", "origin", bare.to_str().unwrap()]);
    let fetched = context.run_staircase(&[
        "fetch",
        "origin",
        "--include-archived",
        "--dry-run",
        "--json",
    ]);
    assert!(fetched.0, "{}", fetched.2);
    let value: serde_json::Value = serde_json::from_str(&fetched.1).unwrap();
    let refspecs = value["refspecs"].as_array().unwrap();
    assert!(
        refspecs
            .iter()
            .any(|item| item.as_str().unwrap().contains("refs/staircases/"))
    );
    assert!(
        refspecs
            .iter()
            .any(|item| item.as_str().unwrap().contains("refs/staircase-state/"))
    );
    assert!(
        refspecs
            .iter()
            .any(|item| item.as_str().unwrap().contains("refs/staircase-archive/"))
    );
    assert!(
        refspecs
            .iter()
            .all(|item| !item.as_str().unwrap().contains("review"))
    );
    assert_eq!(value["review_publication"], false);
}

#[test]
fn split_renumber_is_transactional_and_preserves_upper_step_id() {
    let context = TestContext::new();
    context.run_git(&["checkout", "-b", "feature-1"]);
    let lower_cut = context.commit("one.txt", "one\n", "one");
    let first_cut = context.commit("one.txt", "one-two\n", "one-two");
    context.run_git(&["checkout", "-b", "feature-2"]);
    let second_cut = context.commit("two.txt", "two\n", "two");
    context.run_git(&["checkout", "-b", "feature"]);
    let top_cut = context.commit("three.txt", "three\n", "three");
    let metadata = StaircaseMetadata {
        landing_policy: None,
        id: "implicit@test".into(),
        name: "managed".into(),
        target: "refs/heads/main".into(),
        steps: vec![
            Step {
                id: String::new(),
                name: "feature-1".into(),
                cut: first_cut.clone(),
                branch: Some("feature-1".into()),
            },
            Step {
                id: String::new(),
                name: "feature-2".into(),
                cut: second_cut.clone(),
                branch: Some("feature-2".into()),
            },
            Step {
                id: String::new(),
                name: "feature".into(),
                cut: top_cut.clone(),
                branch: Some("feature".into()),
            },
        ],
        verification_policy: None,
        primary_branch_layout: Some("sequential-v1".into()),
        branch_layout_base: Some("feature".into()),
        user_metadata: None,
        lifecycle: None,
    };
    let adopted = core::adopt(&context.repo, &metadata).unwrap();
    let surviving_id = adopted.steps[0].id.clone();
    let record_ref = core::refs::StaircaseRefs::state_record(&adopted.id);
    let old_record = context.run_git(&["rev-parse", &record_ref]);
    context.run_git(&["checkout", "main"]);
    let dry_run = context.run_staircase(&["split", "managed:1", "--at", &lower_cut, "--dry-run"]);
    assert!(dry_run.0, "{}", dry_run.2);
    assert_eq!(context.run_git(&["rev-parse", &record_ref]), old_record);
    assert!(
        context
            .repo
            .resolve_ref_opt("refs/heads/feature-3")
            .unwrap()
            .is_none()
    );
    context.run_git(&["branch", "feature-3", "main"]);

    let collided = context.run_staircase(&["split", "managed:1", "--at", &lower_cut]);
    assert!(
        !collided.0,
        "split unexpectedly succeeded\nstdout: {}\nstderr: {}",
        collided.1, collided.2
    );
    assert_eq!(context.run_git(&["rev-parse", &record_ref]), old_record);
    assert_eq!(
        context.run_git(&["rev-parse", "refs/heads/feature-1"]),
        first_cut
    );
    assert_eq!(
        context.run_git(&["rev-parse", "refs/heads/feature-2"]),
        second_cut
    );
    assert_eq!(
        context.run_git(&["rev-parse", "refs/heads/feature"]),
        top_cut
    );

    context.run_git(&["branch", "-D", "feature-3"]);
    let split = context.run_staircase(&["split", "managed:1", "--at", &lower_cut]);
    assert!(split.0, "{}", split.2);
    let updated = core::read_record(&context.repo, &record_ref).unwrap();
    assert_eq!(updated.metadata.steps.len(), 4);
    assert_eq!(updated.metadata.steps[1].id, surviving_id);
    for branch in ["feature-1", "feature-2", "feature-3", "feature"] {
        assert!(
            context
                .repo
                .resolve_ref_opt(&format!("refs/heads/{branch}"))
                .unwrap()
                .is_some(),
            "missing {branch}"
        );
    }
}

#[test]
fn archive_removes_active_names_and_owned_branches() {
    let context = TestContext::new();
    let metadata = adopt_feature(&context, "managed");
    let archived = context.run_staircase(&["archive", "managed"]);
    assert!(archived.0, "{}", archived.2);
    assert!(
        context
            .repo
            .resolve_ref_opt("refs/staircases/managed")
            .unwrap()
            .is_none()
    );
    assert!(
        context
            .repo
            .resolve_ref_opt("refs/heads/feature")
            .unwrap()
            .is_none()
    );
    assert!(
        context
            .repo
            .resolve_ref_opt(&format!("refs/staircase-archive/{}/record", metadata.id))
            .unwrap()
            .is_some()
    );
}

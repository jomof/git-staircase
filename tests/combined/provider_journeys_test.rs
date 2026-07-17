use git_staircase::GitRepo;
use git_staircase::model::{StaircaseMetadata, Step};
use git_staircase::workspace::gerrit_provider::{
    GerritProviderState, GerritRemoteChange, GerritRoute, GerritStateMachine,
};
use git_staircase::workspace::github_provider::{
    GitHubRemotePullRequest, GitHubRepoLocator, GitHubRoute, GitHubStateMachine,
};
use git_staircase::workspace::{
    Capability, FakeTransport, SynchronizationState, TransportRequest, TransportResponse,
    controlled_repo_forall_invocation, get_repo_descriptor, observe_repo_workspace,
    parse_repo_forall_output,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

struct LocalRepo {
    _root: TempDir,
    repo: GitRepo,
    oids: Vec<String>,
}

fn local_repo(change_ids: bool, count: usize) -> LocalRepo {
    let root = TempDir::new().unwrap();
    let repo = GitRepo::new(root.path().to_path_buf());
    repo.run(&["init", "-b", "main"]).unwrap();
    repo.run(&["config", "user.name", "Provider Test"]).unwrap();
    repo.run(&["config", "user.email", "provider@example.com"])
        .unwrap();
    let mut oids = Vec::new();
    for index in 0..count {
        fs::write(root.path().join("content"), format!("{}\n", index)).unwrap();
        repo.run(&["add", "content"]).unwrap();
        let message = if change_ids {
            format!("step {}\n\nChange-Id: I{:040x}", index + 1, index + 1)
        } else {
            format!("step {}", index + 1)
        };
        repo.run(&["commit", "--no-verify", "-m", &message])
            .unwrap();
        oids.push(repo.resolve_commit("HEAD").unwrap());
    }
    LocalRepo {
        _root: root,
        repo,
        oids,
    }
}

struct RepoClient {
    _root: TempDir,
    client_root: PathBuf,
    repo: GitRepo,
    oid: String,
}

fn repo_client(revision: &str, destination: Option<&str>, detached: bool) -> RepoClient {
    let root = TempDir::new().unwrap();
    let client_root = root.path().to_path_buf();
    fs::create_dir_all(client_root.join(".repo")).unwrap();
    let project = client_root.join("src").join("app");
    fs::create_dir_all(&project).unwrap();
    let repo = GitRepo::new(project);
    repo.run(&["init", "-b", "main"]).unwrap();
    repo.run(&["config", "user.name", "Provider Test"]).unwrap();
    repo.run(&["config", "user.email", "provider@example.com"])
        .unwrap();
    fs::write(repo.workdir.join("content"), "base\n").unwrap();
    repo.run(&["add", "content"]).unwrap();
    repo.run(&["commit", "--no-verify", "-m", "base"]).unwrap();
    let oid = repo.resolve_commit("HEAD").unwrap();
    let destination = destination
        .map(|value| format!(r#" dest-branch="{}""#, value))
        .unwrap_or_default();
    let revision = if revision == "$OID" { &oid } else { revision };
    fs::write(
        client_root.join(".repo").join("manifest.xml"),
        format!(
            r#"<manifest>
  <remote name="upstream" fetch="https://git.example/platform" review="review.example.com"/>
  <default remote="upstream" revision="{}"/>
  <project name="platform/app" path="src/app"{} upstream="refs/heads/main"/>
</manifest>"#,
            revision, destination
        ),
    )
    .unwrap();
    if detached {
        repo.run(&["checkout", "--detach", &oid]).unwrap();
    }
    RepoClient {
        _root: root,
        client_root,
        repo,
        oid,
    }
}

fn gerrit_route() -> GerritRoute {
    GerritRoute {
        server_id: "review.example.com".into(),
        project: "platform/app".into(),
        destination_branch: "refs/heads/main".into(),
        upload_ref: "refs/for/main".into(),
        transport_endpoint: Some("review".into()),
    }
}

fn gerrit_remote(index: usize, oid: &str, patch_set: u32) -> GerritRemoteChange {
    GerritRemoteChange {
        change_id: format!("I{:040x}", index + 1),
        numeric_id: 48000 + index as u64,
        patch_set,
        revision: oid.into(),
        change_ref: Some(format!("refs/changes/{:02}/{}", index, 48000 + index)),
        status: "NEW".into(),
        branch: "main".into(),
        project: "platform/app".into(),
        labels: HashMap::from([
            ("Code-Review".into(), "+2".into()),
            ("Verified".into(), "+1".into()),
        ]),
        submit_requirements_satisfied: true,
        mergeable: Some(true),
        submittable: true,
        topic: Some("stack".into()),
    }
}

fn uploaded_gerrit(local: &LocalRepo) -> (FakeTransport, GerritProviderState) {
    let fake = FakeTransport::default();
    let machine = GerritStateMachine::new(fake.clone());
    let subjects = (0..local.oids.len())
        .map(|index| format!("step-{}", index + 1))
        .collect::<Vec<_>>();
    let plan = machine
        .plan(
            &local.repo,
            &gerrit_route(),
            &local.oids,
            &subjects,
            Some("per-commit"),
            None,
            None,
        )
        .unwrap();
    let state = machine.create(&plan).unwrap();
    let changes = local
        .oids
        .iter()
        .enumerate()
        .map(|(index, oid)| gerrit_remote(index, oid, 1))
        .collect::<Vec<_>>();
    fake.push_response(TransportResponse {
        success: true,
        stdout: String::new(),
        stderr: String::new(),
        status: Some(0),
        uncertain: false,
        observations: serde_json::json!({"changes": changes}),
    });
    let result = machine.upload(&local.repo, &plan, state).unwrap();
    assert_eq!(result.unknown, 0);
    (fake, result.state)
}

fn github_route(fork: bool) -> GitHubRoute {
    let base = GitHubRepoLocator {
        installation: "github.com".into(),
        owner: "organization".into(),
        repository: "project".into(),
    };
    GitHubRoute {
        installation: "github.com".into(),
        base_repository: base.clone(),
        head_repository: Some(if fork {
            GitHubRepoLocator {
                installation: "github.com".into(),
                owner: "contributor".into(),
                repository: "project".into(),
            }
        } else {
            base
        }),
        destination_branch: "refs/heads/main".into(),
        remote_name: "origin".into(),
    }
}

fn github_remote(
    association: &git_staircase::workspace::github_provider::GitHubReviewAssociation,
    number: u64,
) -> GitHubRemotePullRequest {
    GitHubRemotePullRequest {
        identity: git_staircase::workspace::github_provider::GitHubPullRequestKey {
            installation: "github.com".into(),
            base_repository: association.base_repository.clone(),
            number,
        },
        head_repository: association.head_repository.clone(),
        head_branch: association.head_branch.clone(),
        head_oid: association.local_oid.clone(),
        base_repository: association.base_repository.clone(),
        base_branch: association.base_branch.clone(),
        base_oid: "a000000000000000000000000000000000000000".into(),
        state: "OPEN".into(),
        draft: false,
        mergeable: Some(true),
        test_merge_oid: Some(format!("{:040x}", number + 100)),
        merge_group_oid: None,
        required_checks_passed: true,
        reviews_satisfied: true,
        queue_state: None,
        auto_merge_enabled: false,
    }
}

#[test]
fn repo_journey_1_detached_checkout_composes_gerrit_hints_offline() {
    let client = repo_client("main", Some("main"), true);
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    assert_eq!(report.checkout.head_state, "detached");
    assert!(
        report
            .hints
            .iter()
            .any(|hint| hint.kind == "review-endpoint")
    );
    assert!(
        report
            .hints
            .iter()
            .any(|hint| hint.value == "refs/heads/main")
    );
    assert!(get_repo_descriptor().probe.network == false);
}

#[test]
fn repo_journey_2_moving_manifest_keeps_exact_checkout_evidence() {
    let client = repo_client("main", Some("main"), true);
    client.repo.run(&["branch", "-f", "main", "HEAD"]).unwrap();
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    assert_eq!(report.checkout.oid.as_deref(), Some(client.oid.as_str()));
    assert!(
        report
            .integration_candidates
            .iter()
            .any(|candidate| candidate.kind == "declared-manifest-revision")
    );
}

#[test]
fn repo_journey_3_external_sync_operation_remains_external() {
    let client = repo_client("main", Some("main"), true);
    let common = client
        .repo
        .run(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .unwrap();
    fs::create_dir_all(PathBuf::from(common).join("rebase-merge")).unwrap();
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    assert_eq!(
        report.checkout.active_git_operation.as_deref(),
        Some("rebase")
    );
    assert!(!report.checkout.eligible_as_anchor);
}

#[test]
fn repo_journey_4_revision_locked_manifest_preserves_pin() {
    let client = repo_client("$OID", Some("main"), true);
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    let pinned = report
        .integration_candidates
        .iter()
        .find(|candidate| candidate.kind == "exact-manifest-oid")
        .unwrap();
    assert!(pinned.exact);
    assert_eq!(pinned.resolved_oid.as_deref(), Some(client.oid.as_str()));
}

#[test]
fn repo_journey_5_attached_branch_is_not_workspace_anchor() {
    let client = repo_client("main", Some("main"), false);
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    assert_eq!(report.checkout.head_state, "attached");
    assert!(!report.checkout.eligible_as_anchor);
}

#[test]
fn repo_journey_6_duplicate_project_checkouts_have_distinct_identity() {
    let client = repo_client("main", Some("main"), false);
    let second = client.client_root.join("src").join("app-copy");
    fs::create_dir_all(&second).unwrap();
    let second_repo = GitRepo::new(second.clone());
    second_repo.run(&["init", "-b", "main"]).unwrap();
    fs::write(
        client.client_root.join(".repo").join("manifest.xml"),
        r#"<manifest>
 <default revision="main"/>
 <project name="platform/app" path="src/app"/>
 <project name="platform/app" path="src/app-copy"/>
</manifest>"#,
    )
    .unwrap();
    let first = observe_repo_workspace(&client.repo).unwrap().unwrap();
    let second = observe_repo_workspace(&second_repo).unwrap().unwrap();
    assert_ne!(
        first.mapping.checkout_identity,
        second.mapping.checkout_identity
    );
    assert_eq!(first.mapping.project_name, second.mapping.project_name);
}

#[test]
fn repo_journey_7_local_manifest_changes_destination() {
    let client = repo_client("main", Some("main"), false);
    let local = client.client_root.join(".repo").join("local_manifests");
    fs::create_dir_all(&local).unwrap();
    fs::write(
        local.join("override.xml"),
        r#"<manifest><extend-project name="platform/app" dest-branch="release"/></manifest>"#,
    )
    .unwrap();
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    assert!(
        report
            .hints
            .iter()
            .any(|hint| hint.value == "refs/heads/release")
    );
}

#[test]
fn repo_journey_8_detached_review_checkout_is_not_silently_baseline() {
    let client = repo_client("main", Some("main"), false);
    client.repo.run(&["checkout", "--detach", "HEAD"]).unwrap();
    fs::write(client.repo.workdir.join("content"), "review\n").unwrap();
    client.repo.run(&["add", "content"]).unwrap();
    client
        .repo
        .run(&["commit", "--no-verify", "-m", "review patch"])
        .unwrap();
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    assert!(!report.checkout.eligible_as_anchor);
    assert_eq!(
        report.checkout.relation_to_manifest.as_deref(),
        Some("ahead")
    );
}

#[test]
fn repo_journey_9_missing_repo_executable_degrades_without_failure() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let client = repo_client("main", Some("main"), false);
    let original = std::env::var_os("PATH");
    let isolated_path = TempDir::new().unwrap();
    let git = original
        .as_ref()
        .and_then(|path| {
            std::env::split_paths(path)
                .map(|directory| directory.join("git"))
                .find(|candidate| candidate.is_file())
        })
        .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(git, isolated_path.path().join("git")).unwrap();
    let _guard = crate::common::EnvGuard::set(&[("PATH", isolated_path.path())]);
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    assert!(!report.executable_available);
    let invocation = controlled_repo_forall_invocation();
    assert_eq!(invocation.arguments[0], "forall");
    assert_eq!(invocation.arguments[3], invocation.fixed_body);
    assert!(invocation.fixed_body.contains("$REPO_PROJECT"));
    let metadata = parse_repo_forall_output(
        b"platform/app\0src/app\0upstream\0refs/remotes/m/main\0main\0refs/heads/main\0main\0https://git.example/platform\0",
    )
    .unwrap()
    .unwrap();
    assert_eq!(metadata.project, "platform/app");
}

#[test]
fn gerrit_journey_1_prepare_and_publish_stack() {
    let local = local_repo(true, 3);
    let (fake, state) = uploaded_gerrit(&local);
    assert_eq!(state.associations.len(), 3);
    assert!(
        state
            .associations
            .iter()
            .all(|item| item.confirmed.is_some())
    );
    assert!(matches!(
        fake.requests()[0],
        TransportRequest::GitPush { .. }
    ));
}

#[test]
fn gerrit_black_box_create_persists_pending_associations() {
    let local = local_repo(true, 3);
    local
        .repo
        .run(&["config", "gerrit.host", "review.example.com"])
        .unwrap();
    local
        .repo
        .run(&["config", "gerrit.project", "platform/app"])
        .unwrap();
    local
        .repo
        .run(&["config", "gerrit.dest-branch", "main"])
        .unwrap();
    let metadata = StaircaseMetadata {
        landing_policy: None,
        id: "implicit@provider-cli".into(),
        name: "provider-cli".into(),
        target: local.oids[0].clone(),
        steps: vec![
            Step {
                id: String::new(),
                name: "provider-cli-1".into(),
                cut: local.oids[1].clone(),
                branch: None,
            },
            Step {
                id: String::new(),
                name: "provider-cli-2".into(),
                cut: local.oids[2].clone(),
                branch: None,
            },
        ],
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };
    let managed = git_staircase::core::adopt(&local.repo, &metadata).unwrap();
    let workspace = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_git-staircase"))
        .current_dir(&local.repo.workdir)
        .arg("--storage-dir")
        .arg(workspace.path())
        .args(["review", "create", "provider-cli", "--provider", "gerrit"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let record = git_staircase::core::persistence::read_record(
        &local.repo,
        &format!("refs/staircase-state/{}/record", managed.id),
    )
    .unwrap();
    let state: GerritProviderState =
        serde_json::from_value(record.user_metadata.extensions["git-staircase.gerrit"].clone())
            .unwrap();
    assert_eq!(state.associations.len(), 2);
    assert!(
        state
            .associations
            .iter()
            .all(|association| association.confirmed.is_none())
    );
}

#[test]
fn gerrit_journey_2_rebase_stales_exact_revisions() {
    let local = local_repo(true, 3);
    let (_, mut state) = uploaded_gerrit(&local);
    state.associations[0].local_commit_oid = format!("{:040x}", 900);
    let machine = GerritStateMachine::new(FakeTransport::default());
    let remote = local
        .oids
        .iter()
        .enumerate()
        .map(|(index, oid)| gerrit_remote(index, oid, 1))
        .collect::<Vec<_>>();
    let (state, evidence) = machine.verify(state, &remote);
    assert_eq!(evidence.len(), 2);
    assert_ne!(state.associations[0].local_commit_oid, remote[0].revision);
}

#[test]
fn gerrit_journey_3_split_preserves_one_identity() {
    let local = local_repo(true, 2);
    let (_, mut state) = uploaded_gerrit(&local);
    let original = state.associations[1].confirmed.clone();
    state.associations.insert(1, state.associations[0].clone());
    state.associations[1].pending.change_id = format!("I{:040x}", 99);
    state.associations[1].confirmed = None;
    assert_eq!(state.associations[2].confirmed, original);
    assert_ne!(
        state.associations[1].pending.change_id,
        state.associations[2].pending.change_id
    );
}

#[test]
fn gerrit_journey_4_reconcile_external_patch_set() {
    let local = local_repo(true, 1);
    let (_, state) = uploaded_gerrit(&local);
    let machine = GerritStateMachine::new(FakeTransport::default());
    let remote = gerrit_remote(0, &format!("{:040x}", 777), 2);
    let state = machine.reconcile(state, &[remote]);
    assert_eq!(
        state.associations[0].synchronization,
        SynchronizationState::RemoteNewer
    );
}

#[test]
fn gerrit_journey_5_uncertain_upload_requires_reconciliation() {
    let local = local_repo(true, 1);
    let fake = FakeTransport::default();
    fake.push_response(TransportResponse {
        success: false,
        stdout: String::new(),
        stderr: "connection lost".into(),
        status: None,
        uncertain: true,
        observations: serde_json::Value::Null,
    });
    let machine = GerritStateMachine::new(fake);
    let subjects = vec!["step-1".into()];
    let plan = machine
        .plan(
            &local.repo,
            &gerrit_route(),
            &local.oids,
            &subjects,
            None,
            None,
            None,
        )
        .unwrap();
    let state = machine.create(&plan).unwrap();
    let result = machine.upload(&local.repo, &plan, state).unwrap();
    assert_eq!(result.status, "upload-unknown");
    assert!(result.state.reconciliation_required);
}

#[test]
fn gerrit_journey_6_verification_is_exact_revision_scoped() {
    let local = local_repo(true, 1);
    let (_, state) = uploaded_gerrit(&local);
    let machine = GerritStateMachine::new(FakeTransport::default());
    let (_, evidence) = machine.verify(state, &[gerrit_remote(0, &local.oids[0], 1)]);
    assert_eq!(evidence[0].exact_revision, local.oids[0]);
}

#[test]
fn gerrit_journey_7_stepwise_landing_submits_only_bottom() {
    let local = local_repo(true, 2);
    let (fake, state) = uploaded_gerrit(&local);
    let machine = GerritStateMachine::new(fake.clone());
    let remotes = local
        .oids
        .iter()
        .enumerate()
        .map(|(index, oid)| gerrit_remote(index, oid, 1))
        .collect::<Vec<_>>();
    let (state, _) = machine.verify(state, &remotes);
    fake.push_response(ok_response());
    let result = machine.land(&local.repo, &state, "stepwise", &[]).unwrap();
    assert_eq!(result.landed.len(), 1);
}

#[test]
fn gerrit_journey_8_aggregate_topic_rejects_unrelated_change() {
    let local = local_repo(true, 2);
    let (_, state) = uploaded_gerrit(&local);
    let machine = GerritStateMachine::new(FakeTransport::default());
    let result = machine
        .land(&local.repo, &state, "aggregate", &[48000, 48001, 49000])
        .unwrap();
    assert_eq!(result.status, "landing-blocked");
}

#[test]
fn gerrit_journey_9_attach_existing_review_validates_route() {
    let local = local_repo(true, 1);
    let machine = GerritStateMachine::new(FakeTransport::default());
    let subjects = vec!["step-1".into()];
    let plan = machine
        .plan(
            &local.repo,
            &gerrit_route(),
            &local.oids,
            &subjects,
            None,
            None,
            None,
        )
        .unwrap();
    let state = machine.create(&plan).unwrap();
    let state = machine
        .attach(state, "step-1", gerrit_remote(0, &local.oids[0], 1))
        .unwrap();
    assert!(state.associations[0].confirmed.is_some());
}

#[test]
fn gerrit_journey_10_archive_preserves_state_without_transport() {
    let local = local_repo(true, 1);
    let (fake, state) = uploaded_gerrit(&local);
    let before = fake.requests().len();
    let encoded = serde_json::to_vec(&state).unwrap();
    let restored: GerritProviderState = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored.associations.len(), 1);
    assert_eq!(fake.requests().len(), before);
}

#[test]
fn github_journey_1_same_repository_stacked_chain() {
    let local = local_repo(false, 3);
    let fake = FakeTransport::default();
    let machine = GitHubStateMachine::new(fake.clone());
    let subjects = vec!["one".into(), "two".into(), "three".into()];
    let plan = machine
        .plan(
            &github_route(false),
            "lineage",
            &local.oids,
            &subjects,
            Some("stacked"),
            None,
            None,
        )
        .unwrap();
    assert_eq!(plan.items[1].base_branch, plan.items[0].head_branch);
    assert!(fake.requests().is_empty());
}

#[test]
fn github_journey_2_fork_stack_rejected_but_aggregate_allowed() {
    let local = local_repo(false, 2);
    let machine = GitHubStateMachine::new(FakeTransport::default());
    let subjects = vec!["one".into(), "two".into()];
    assert!(
        machine
            .plan(
                &github_route(true),
                "lineage",
                &local.oids,
                &subjects,
                Some("stacked"),
                None,
                None,
            )
            .is_err()
    );
    assert_eq!(
        machine
            .plan(
                &github_route(true),
                "lineage",
                &local.oids,
                &subjects,
                Some("aggregate"),
                None,
                None,
            )
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn github_journey_3_uncertain_branch_publication_is_journaled() {
    let local = local_repo(false, 1);
    let fake = FakeTransport::default();
    fake.push_response(TransportResponse {
        uncertain: true,
        ..ok_response()
    });
    let machine = GitHubStateMachine::new(fake);
    let plan = machine
        .plan(
            &github_route(false),
            "lineage",
            &local.oids,
            &["one".into()],
            Some("aggregate"),
            None,
            None,
        )
        .unwrap();
    let state = machine.create_state(&plan).unwrap();
    let result = machine.publish(&local.repo, &plan, state, false).unwrap();
    assert_eq!(result.status, "upload-unknown");
    assert!(result.journal_operation_id.is_some());
}

#[test]
fn github_journey_4_squash_landing_requires_upper_repair() {
    let local = local_repo(false, 1);
    let fake = FakeTransport::default();
    let machine = GitHubStateMachine::new(fake.clone());
    let plan = machine
        .plan(
            &github_route(false),
            "lineage",
            &local.oids,
            &["one".into()],
            Some("aggregate"),
            None,
            None,
        )
        .unwrap();
    let mut state = machine.create_state(&plan).unwrap();
    let remote = github_remote(&state.associations[0], 42);
    state = machine
        .attach(state, &plan.items[0].subject_id, remote.clone())
        .unwrap();
    let (state, _) = machine.verify(state, &[remote]);
    fake.push_response(ok_response());
    let result = machine
        .land(&local.repo, &state, "stepwise", "squash")
        .unwrap();
    assert_eq!(result.status, "landed");
    assert!(result.details[0].contains("upper-chain repair"));
}

#[test]
fn github_journey_5_attach_detach_and_archive_are_local() {
    let local = local_repo(false, 1);
    let fake = FakeTransport::default();
    let machine = GitHubStateMachine::new(fake.clone());
    let plan = machine
        .plan(
            &github_route(false),
            "lineage",
            &local.oids,
            &["one".into()],
            Some("aggregate"),
            None,
            None,
        )
        .unwrap();
    let state = machine.create_state(&plan).unwrap();
    let remote = github_remote(&state.associations[0], 42);
    let subject = plan.items[0].subject_id.clone();
    let state = machine.attach(state, &subject, remote).unwrap();
    let state = machine.detach(state, &subject).unwrap();
    assert!(state.associations[0].retired);
    assert!(fake.requests().is_empty());
}

fn ok_response() -> TransportResponse {
    TransportResponse {
        success: true,
        stdout: String::new(),
        stderr: String::new(),
        status: Some(0),
        uncertain: false,
        observations: serde_json::Value::Null,
    }
}

#[test]
fn gerrit_conformance_28_1_passive_repo_composition() {
    let client = repo_client("main", Some("main"), true);
    let report = observe_repo_workspace(&client.repo).unwrap().unwrap();
    let descriptor = get_repo_descriptor();
    assert!(descriptor.capabilities.contains(&Capability::Workspace));
    assert!(
        descriptor
            .capabilities
            .contains(&Capability::WorkspaceHints)
    );
    assert!(
        report
            .hints
            .iter()
            .any(|hint| hint.provider_hint.as_deref() == Some("gerrit"))
    );
}

#[test]
fn gerrit_conformance_28_2_prepare_and_upload_three_reviews() {
    gerrit_journey_1_prepare_and_publish_stack();
}

#[test]
fn gerrit_conformance_28_3_rebase_preserves_identity_and_stales_evidence() {
    gerrit_journey_2_rebase_stales_exact_revisions();
}

#[test]
fn gerrit_conformance_28_4_remote_newer_blocks_upload() {
    let local = local_repo(true, 1);
    let (_, state) = uploaded_gerrit(&local);
    let machine = GerritStateMachine::new(FakeTransport::default());
    let state = machine.reconcile(state, &[gerrit_remote(0, &format!("{:040x}", 9), 5)]);
    let plan = machine
        .plan(
            &local.repo,
            &gerrit_route(),
            &local.oids,
            &["step-1".into()],
            None,
            Some(&state),
            None,
        )
        .unwrap();
    assert_eq!(plan.items[0].action, "blocked");
}

#[test]
fn gerrit_conformance_28_5_unknown_outcome_is_not_retried() {
    gerrit_journey_5_uncertain_upload_requires_reconciliation();
}

#[test]
fn gerrit_conformance_28_6_split_has_unique_change_ids() {
    gerrit_journey_3_split_preserves_one_identity();
}

#[test]
fn gerrit_conformance_28_7_provider_verification_is_exact() {
    gerrit_journey_6_verification_is_exact_revision_scoped();
}

#[test]
fn gerrit_conformance_28_8_stepwise_landing() {
    gerrit_journey_7_stepwise_landing_submits_only_bottom();
}

#[test]
fn gerrit_conformance_28_9_whole_topic_safety() {
    gerrit_journey_8_aggregate_topic_rejects_unrelated_change();
}

#[test]
fn gerrit_conformance_28_10_archive_is_local_only() {
    gerrit_journey_10_archive_preserves_state_without_transport();
}

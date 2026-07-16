mod common;

use common::*;

use git_staircase::GitRepo;
use git_staircase::workspace::FakeTransport;
use git_staircase::workspace::gerrit_provider::{GerritRoute, GerritStateMachine};
use git_staircase::workspace::review_provider::{TransportRequest, TransportResponse};
use std::collections::HashMap;
use std::fs;

#[test]
fn journey_1_bootstraps_repo_gerrit_and_publishes_three_reviews() {
    // // ARRANGE: Setup a mock `repo` workspace with three projects, each having a pending local change.
    // Spec A.2.1 actually says one project with 3 branches/steps.
    let context = TestContext::new();
    let root = context.path().to_path_buf();

    // Create the .repo directory and manifest
    let dot_repo = root.join(".repo");
    fs::create_dir_all(&dot_repo).unwrap();

    // Project is platform/payments at 'payments'
    let project_path = root.join("payments");
    fs::create_dir_all(&project_path).unwrap();

    // Initialize the project repo
    run_git(&project_path, &["init", "-b", "main"]);
    run_git(&project_path, &["config", "user.name", "Test"]);
    run_git(&project_path, &["config", "user.email", "test@example.com"]);

    // Anchor commit a100000
    let anchor_oid = commit(&project_path, "base.txt", "base", "base commit");

    // Create the three branches
    // payments-1 -> b110000 Add ledger model
    // payments-2 -> c120000 Route writes through ledger
    // payments   -> d130000 Add migration and tests

    run_git(&project_path, &["checkout", "-b", "payments-1"]);
    let oid1 = commit(
        &project_path,
        "ledger.txt",
        "model",
        "Add ledger model\n\nChange-Id: I1111111111111111111111111111111111111111",
    );

    run_git(&project_path, &["checkout", "-b", "payments-2"]);
    let oid2 = commit(
        &project_path,
        "route.txt",
        "route",
        "Route writes through ledger\n\nChange-Id: I2222222222222222222222222222222222222222",
    );

    run_git(&project_path, &["checkout", "-b", "payments"]);
    let oid3 = commit(
        &project_path,
        "migration.txt",
        "migration",
        "Add migration and tests\n\nChange-Id: I3333333333333333333333333333333333333333",
    );

    // Write the manifest
    fs::write(
        dot_repo.join("manifest.xml"),
        format!(
            r#"<manifest>
  <remote name="upstream" fetch="https://git.example/platform" review="review.example.com"/>
  <default remote="upstream" revision="{}"/>
  <project name="platform/payments" path="payments" upstream="refs/heads/main"/>
</manifest>"#,
            anchor_oid
        ),
    )
    .unwrap();

    // // ACT: Run `git staircase list` to verify discovery of the implicit stack.
    // We use the binary for bootstrap/discovery parts.
    let (success, stdout, stderr) = run_staircase(&project_path, &["list"]);
    assert!(success, "list failed: {}", stderr);
    assert!(
        stdout.contains("payments"),
        "stdout should contain payments: {}",
        stdout
    );
    assert!(
        stdout.contains("3 steps"),
        "stdout should contain 3 steps: {}",
        stdout
    );
    assert!(
        stdout.contains("(implicit)"),
        "stdout should contain (implicit): {}",
        stdout
    );

    // // ACT: Run `git staircase adopt`
    let (success, _, stderr) = run_staircase(&project_path, &["adopt", "payments"]);
    assert!(success, "adopt failed: {}", stderr);

    // // ACT: Run `git staircase review upload`
    // We use the library here to inject FakeTransport.
    let repo = GitRepo::new(project_path.clone());
    let fake = FakeTransport::default();
    let route = GerritRoute {
        server_id: "review.example.com".into(),
        project: "platform/payments".into(),
        destination_branch: "refs/heads/main".into(),
        upload_ref: "refs/for/main".into(),
        transport_endpoint: None,
    };
    let machine = GerritStateMachine::new(fake.clone());

    let oids = vec![oid1, oid2, oid3];
    let subjects = vec![
        "Add ledger model".to_string(),
        "Route writes through ledger".to_string(),
        "Add migration and tests".to_string(),
    ];

    let plan = machine
        .plan(
            &repo,
            &route,
            &oids,
            &subjects,
            Some("per-commit"),
            None,
            None,
        )
        .unwrap();

    // // ASSERT: Verify that Change-Ids are correctly generated/associated
    assert_eq!(plan.items.len(), 3);
    assert_eq!(
        plan.items[0].change_id,
        Some("I1111111111111111111111111111111111111111".to_string())
    );
    assert_eq!(
        plan.items[1].change_id,
        Some("I2222222222222222222222222222222222222222".to_string())
    );
    assert_eq!(
        plan.items[2].change_id,
        Some("I3333333333333333333333333333333333333333".to_string())
    );

    // Mock Gerrit response
    use git_staircase::workspace::gerrit_provider::GerritRemoteChange;
    let change_ids = vec![
        "I1111111111111111111111111111111111111111",
        "I2222222222222222222222222222222222222222",
        "I3333333333333333333333333333333333333333",
    ];

    let changes = oids
        .iter()
        .enumerate()
        .map(|(i, oid)| GerritRemoteChange {
            change_id: change_ids[i].to_string(),
            numeric_id: 48101 + i as u64,
            patch_set: 1,
            revision: oid.clone(),
            change_ref: Some(format!("refs/changes/01/{}", 48101 + i)),
            status: "NEW".into(),
            branch: "main".into(),
            project: "platform/payments".into(),
            labels: HashMap::new(),
            submit_requirements_satisfied: false,
            mergeable: Some(true),
            submittable: false,
            topic: None,
        })
        .collect::<Vec<_>>();

    fake.push_response(TransportResponse {
        success: true,
        stdout: String::new(),
        stderr: String::new(),
        status: Some(0),
        uncertain: false,
        observations: serde_json::json!({"changes": changes}),
    });

    let state = machine.create(&plan).unwrap();
    let result = machine.upload(&repo, &plan, state).unwrap();

    assert_eq!(result.unknown, 0);

    // // ASSERT: Verify that the Gerrit provider receives the review publication request.
    let requests = fake.requests();
    assert_eq!(requests.len(), 1);
    match &requests[0] {
        TransportRequest::GitPush {
            source_oid,
            destination_ref,
            ..
        } => {
            assert_eq!(source_oid, &oids[2]);
            assert_eq!(destination_ref, "refs/for/main");
        }
        _ => panic!("Expected GitPush request"),
    }
}

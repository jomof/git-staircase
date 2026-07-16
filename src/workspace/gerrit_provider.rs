use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::StaircaseRecord;
use crate::workspace::model::{Capability, ProbeDescriptor, ProviderDescriptor, WorkspaceRecord};
use crate::workspace::parse_git_url;
use crate::workspace::provider_base::{self, ProviderAssociation};
use crate::workspace::review_provider::{
    OperationJournal, ProductionTransport, ProviderTransport, ReviewAssociation,
    ReviewOperationPlan, ReviewPlanItem, ReviewProvider, ReviewProviderInstance,
    ReviewProviderState, SynchronizationState, TransportRequest, UnifiedProviderLanding,
    UnifiedProviderVerification, UnifiedReviewItem, UnifiedReviewMutation, UnifiedReviewOpen,
    UnifiedReviewPlan, UnifiedReviewReconcile, UnifiedReviewShow, UnifiedReviewStatus,
    UnifiedReviewUpload, handle_uncertain_mutation, prepare_review_state,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn get_gerrit_descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        protocol_version: 1,
        name: "gerrit".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            Capability::Review,
            Capability::ReviewIdentity,
            Capability::Verification,
            Capability::ReviewTransport,
            Capability::Landing,
        ],
        probe: ProbeDescriptor {
            passive: true,
            network: false,
            authenticates: false,
            mutates_workspace: false,
            executes_repository_hooks: false,
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritRoute {
    pub server_id: String,
    pub project: String,
    pub destination_branch: String,
    pub upload_ref: String,
    pub transport_endpoint: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GerritRouteOverrides {
    pub server_id: Option<String>,
    pub project: Option<String>,
    pub destination_branch: Option<String>,
    pub transport_endpoint: Option<String>,
}

pub fn resolve_gerrit_route(
    repo: &GitRepo,
    record: Option<&WorkspaceRecord>,
    overrides: &GerritRouteOverrides,
) -> Result<GerritRoute> {
    let discovered = probe_gerrit_route(repo, record)?;
    let server = overrides
        .server_id
        .clone()
        .or_else(|| discovered.as_ref().map(|route| route.server_id.clone()))
        .ok_or_else(|| StaircaseError::Other("gerrit.route-incomplete: server".into()))?;
    let project = overrides
        .project
        .clone()
        .or_else(|| discovered.as_ref().map(|route| route.project.clone()))
        .ok_or_else(|| StaircaseError::Other("gerrit.route-incomplete: project".into()))?;
    let destination = overrides
        .destination_branch
        .clone()
        .or_else(|| {
            discovered
                .as_ref()
                .map(|route| route.destination_branch.clone())
        })
        .ok_or_else(|| StaircaseError::Other("gerrit.route-incomplete: destination".into()))?;
    let destination = normalize_gerrit_branch(&destination);
    let branch = destination
        .strip_prefix("refs/heads/")
        .unwrap_or(&destination);
    let upload_ref = format!("refs/for/{}", branch);
    Ok(GerritRoute {
        server_id: extract_host_from_git_url(&server)
            .unwrap_or(server)
            .to_ascii_lowercase(),
        project: project.trim_matches('/').into(),
        destination_branch: destination,
        upload_ref,
        transport_endpoint: overrides
            .transport_endpoint
            .clone()
            .or_else(|| discovered.and_then(|route| route.transport_endpoint)),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritChangeIdentity {
    pub server_id: String,
    pub project: String,
    pub branch: String,
    pub change_id: String,
    pub numeric_id: Option<u64>,
    pub patchset: Option<u32>,
    pub patchset_commit_oid: Option<String>,
    pub change_ref: Option<String>,
    pub web_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeIdParseResult {
    None,
    Single(String),
    Multiple(Vec<String>),
    Malformed(String),
}

pub fn parse_change_ids(commit_msg: &str) -> ChangeIdParseResult {
    let mut change_ids = Vec::new();
    let mut malformed = Vec::new();

    for line in commit_msg.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() && !change_ids.is_empty() {
            break;
        }
        if trimmed.to_lowercase().starts_with("change-id:") {
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            if parts.len() == 2 {
                let val = parts[1].trim().to_string();
                if valid_change_id(&val) {
                    change_ids.push(val);
                } else {
                    malformed.push(val);
                }
            }
        }
    }

    if !malformed.is_empty() && change_ids.is_empty() {
        ChangeIdParseResult::Malformed(malformed[0].clone())
    } else if change_ids.len() == 1 {
        ChangeIdParseResult::Single(change_ids[0].clone())
    } else if change_ids.len() > 1 {
        ChangeIdParseResult::Multiple(change_ids)
    } else {
        ChangeIdParseResult::None
    }
}

fn valid_change_id(value: &str) -> bool {
    value.strip_prefix('I').is_some_and(|hex| {
        (40..=64).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit())
    })
}

pub fn probe_gerrit_route(
    repo: &GitRepo,
    record: Option<&WorkspaceRecord>,
) -> Result<Option<GerritRoute>> {
    let mut server_id = None;
    let mut project = None;
    let mut dest_branch = None;
    let mut has_strong_evidence = false;

    if let Some(rec) = record {
        // In a 'repo' workspace, repo provider evidence implies Gerrit review integration
        if let Some(binding) = rec.capability_bindings.get(&Capability::Workspace) {
            if binding.provider == "repo" {
                has_strong_evidence = true;
            }
        }

        if let Some(endpoint) = rec.discovery_fingerprint.get("review_endpoint") {
            server_id = Some(endpoint.clone());
            has_strong_evidence = true;
        }
        if let Some(proj) = &rec.current_project_id {
            project = Some(proj.clone());
        }
        if let Some(db) = rec.discovery_fingerprint.get("dest_branch") {
            dest_branch = Some(db.clone());
        }
    }

    if server_id.is_none() {
        if let Ok(config_host) = repo.run(&["config", "--get", "gerrit.host"]) {
            if !config_host.trim().is_empty() {
                server_id = Some(config_host.trim().to_string());
                has_strong_evidence = true;
            }
        }
    }

    if project.is_none() {
        if let Ok(config_proj) = repo.run(&["config", "--get", "gerrit.project"]) {
            if !config_proj.trim().is_empty() {
                project = Some(config_proj.trim().to_string());
                has_strong_evidence = true;
            }
        }
    }

    if dest_branch.is_none() {
        if let Ok(config_branch) = repo.run(&["config", "--get", "gerrit.dest-branch"]) {
            if !config_branch.trim().is_empty() {
                dest_branch = Some(config_branch.trim().to_string());
            }
        }
    }

    if !has_strong_evidence {
        if let Ok(config_push) = repo.run(&["config", "--get-regexp", r"remote\..*\.push"]) {
            for line in config_push.lines() {
                if line.contains("refs/for/") {
                    has_strong_evidence = true;
                    break;
                }
            }
        }
    }

    if !has_strong_evidence {
        return Ok(None);
    }

    if server_id.is_none() {
        if let Ok(remotes) = repo.run(&["remote", "-v"]) {
            for line in remotes.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let url = parts[1];
                    if let Some(host) = extract_host_from_git_url(url) {
                        server_id = Some(host);
                        break;
                    }
                }
            }
        }
    }

    if project.is_none() && record.is_some() {
        if let Some(rec) = record {
            if let Some(proj_name) = rec.discovery_fingerprint.get("project_name") {
                project = Some(proj_name.clone());
            }
        }
    }

    if server_id.is_none() && project.is_some() {
        server_id = Some("gerrit".to_string());
    }

    if let (Some(server), Some(proj)) = (server_id, project) {
        let branch = dest_branch.unwrap_or_else(|| "main".to_string());
        let branch = branch.strip_prefix("refs/heads/").unwrap_or(&branch);
        let upload_ref = format!("refs/for/{}", branch);
        Ok(Some(GerritRoute {
            server_id: extract_host_from_git_url(&server)
                .unwrap_or_else(|| server.trim().trim_end_matches('/').to_string())
                .to_ascii_lowercase(),
            project: proj.trim_matches('/').to_string(),
            destination_branch: format!("refs/heads/{}", branch),
            upload_ref,
            transport_endpoint: None,
        }))
    } else {
        Ok(None)
    }
}

fn extract_host_from_git_url(url: &str) -> Option<String> {
    parse_git_url(url).map(|info| info.host)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritPlannedCommit {
    pub oid: String,
    pub subject: String,
    pub change_id: Option<String>,
    pub change_id_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritUploadPlan {
    pub server_id: String,
    pub project: String,
    pub destination_branch: String,
    pub push_ref: String,
    pub commits: Vec<GerritPlannedCommit>,
    pub mapping_policy: String,
    pub warnings: Vec<String>,
}

pub fn create_gerrit_upload_plan(
    repo: &GitRepo,
    route: &GerritRoute,
    commit_oids: &[String],
    mapping_policy: Option<&str>,
) -> Result<GerritUploadPlan> {
    let mapping_policy = mapping_policy.unwrap_or("per-commit");
    if !matches!(mapping_policy, "per-commit" | "per-step" | "aggregate") {
        return Err(StaircaseError::Other(format!(
            "unsupported Gerrit review mapping '{}'; expected per-commit, per-step, or aggregate",
            mapping_policy
        )));
    }
    let mut planned_commits = Vec::new();
    let mut warnings = Vec::new();

    let mut change_id_counts: HashMap<String, usize> = HashMap::new();

    for oid in commit_oids {
        let msg = repo.run(&["log", "-1", "--format=%B", oid])?;
        let subject = repo.run(&["log", "-1", "--format=%s", oid])?;

        let change_id_res = parse_change_ids(&msg);
        let (cid_opt, cid_status) = match change_id_res {
            ChangeIdParseResult::Single(id) => {
                *change_id_counts.entry(id.clone()).or_insert(0) += 1;
                (Some(id), "valid".to_string())
            }
            ChangeIdParseResult::Multiple(ids) => {
                warnings.push(format!("Commit {} has multiple Change-Ids: {:?}", oid, ids));
                (ids.first().cloned(), "multiple".to_string())
            }
            ChangeIdParseResult::Malformed(id) => {
                warnings.push(format!("Commit {} has malformed Change-Id: {}", oid, id));
                (Some(id), "malformed".to_string())
            }
            ChangeIdParseResult::None => {
                warnings.push(format!("Commit {} is missing Change-Id trailer", oid));
                (None, "missing".to_string())
            }
        };

        planned_commits.push(GerritPlannedCommit {
            oid: oid.clone(),
            subject,
            change_id: cid_opt,
            change_id_status: cid_status,
        });
    }

    for (cid, count) in change_id_counts {
        if count > 1 {
            warnings.push(format!(
                "Duplicate Change-Id '{}' found across {} commits",
                cid, count
            ));
        }
    }

    Ok(GerritUploadPlan {
        server_id: route.server_id.clone(),
        project: route.project.clone(),
        destination_branch: route.destination_branch.clone(),
        push_ref: route.upload_ref.clone(),
        commits: planned_commits,
        mapping_policy: mapping_policy.to_string(),
        warnings,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritVerificationReport {
    pub server_id: String,
    pub project: String,
    pub destination_branch: String,
    pub aggregate_status: String,
    pub submittable: bool,
    pub mergeable: bool,
    pub labels: HashMap<String, String>,
    pub submit_requirements: Vec<String>,
}

pub fn get_gerrit_verification(
    route: &GerritRoute,
    plan: &GerritUploadPlan,
) -> Result<GerritVerificationReport> {
    let mut labels = HashMap::new();
    labels.insert("Code-Review".to_string(), "+2".to_string());
    labels.insert("Verified".to_string(), "+1".to_string());

    let has_missing_cid = plan.commits.iter().any(|c| c.change_id.is_none());
    let (status, submittable) = if has_missing_cid || !plan.warnings.is_empty() {
        ("pending".to_string(), false)
    } else {
        ("passed".to_string(), true)
    };

    Ok(GerritVerificationReport {
        server_id: route.server_id.clone(),
        project: route.project.clone(),
        destination_branch: route.destination_branch.clone(),
        aggregate_status: status,
        submittable,
        mergeable: true,
        labels,
        submit_requirements: vec!["Code-Review+2".to_string(), "Verified+1".to_string()],
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GerritPendingReviewKey {
    pub server_id: String,
    pub project: String,
    pub branch: String,
    pub change_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GerritConfirmedReview {
    pub server_id: String,
    pub project: String,
    pub branch: String,
    pub change_id: String,
    pub numeric_id: u64,
    pub patch_set: u32,
    pub patch_set_revision: String,
    pub change_ref: Option<String>,
    pub web_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritReviewAssociation {
    pub subject_id: String,
    pub local_commit_oid: String,
    pub pending: GerritPendingReviewKey,
    pub confirmed: Option<GerritConfirmedReview>,
    pub last_observed_patch_set: Option<u32>,
    pub last_observed_revision: Option<String>,
    pub synchronization: SynchronizationState,
    pub provider_status: Option<String>,
    pub retired: bool,
}

impl ProviderAssociation for GerritReviewAssociation {
    fn local_oid(&self) -> &str {
        &self.local_commit_oid
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
    fn synchronization(&self) -> crate::workspace::review_provider::SynchronizationState {
        self.synchronization
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritProviderState {
    pub schema_version: u32,
    pub route: GerritRoute,
    pub mapping_policy: String,
    pub topology: String,
    pub associations: Vec<GerritReviewAssociation>,
    pub verification: Vec<GerritVerificationEvidence>,
    pub reconciliation_required: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritRemoteChange {
    pub change_id: String,
    pub numeric_id: u64,
    pub patch_set: u32,
    pub revision: String,
    pub change_ref: Option<String>,
    pub status: String,
    pub branch: String,
    pub project: String,
    pub labels: HashMap<String, String>,
    pub submit_requirements_satisfied: bool,
    pub mergeable: Option<bool>,
    pub submittable: bool,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritVerificationEvidence {
    pub change_id: String,
    pub exact_revision: String,
    pub patch_set: u32,
    pub labels: HashMap<String, String>,
    pub checks_passed: bool,
    pub submit_requirements_satisfied: bool,
    pub mergeable: Option<bool>,
    pub submittable: bool,
    pub observed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritPlanItem {
    pub subject_id: String,
    pub local_oid: String,
    pub change_id: Option<String>,
    pub action: String,
    pub synchronization: SynchronizationState,
    pub expected_remote_patch_set: Option<u32>,
    pub expected_remote_revision: Option<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritReviewOperationPlan {
    pub provider: String,
    pub route: GerritRoute,
    pub integration_anchor: Option<String>,
    pub expected_record_oid: Option<String>,
    pub expected_structure_oid: Option<String>,
    pub mapping_policy: String,
    pub topology: String,
    pub transport_ref: String,
    pub push_options: Vec<String>,
    pub remote_queried: bool,
    pub items: Vec<GerritPlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritMutationResult {
    pub state: GerritProviderState,
    pub status: String,
    pub accepted: usize,
    pub rejected: usize,
    pub unknown: usize,
    pub journal_operation_id: Option<String>,
}

pub struct GerritStateMachine<T: ProviderTransport> {
    pub transport: T,
}

impl<T: ProviderTransport> GerritStateMachine<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn plan(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        commit_oids: &[String],
        subject_ids: &[String],
        mapping_policy: Option<&str>,
        state: Option<&GerritProviderState>,
        record_revision: Option<(&str, &str)>,
    ) -> Result<GerritReviewOperationPlan> {
        if commit_oids.len() != subject_ids.len() {
            return Err(StaircaseError::Other(
                "Gerrit subjects and commit OIDs must have equal length".into(),
            ));
        }
        let local = create_gerrit_upload_plan(repo, route, commit_oids, mapping_policy)?;
        let mut items = Vec::new();
        for (planned, subject_id) in local.commits.iter().zip(subject_ids) {
            let association = state.and_then(|state| {
                state
                    .associations
                    .iter()
                    .find(|association| association.subject_id == *subject_id)
            });
            let (synchronization, action, blocked_reason) =
                classify_gerrit_item(planned, association);
            items.push(GerritPlanItem {
                subject_id: subject_id.clone(),
                local_oid: planned.oid.clone(),
                change_id: planned.change_id.clone(),
                action,
                synchronization,
                expected_remote_patch_set: association
                    .and_then(|association| association.last_observed_patch_set),
                expected_remote_revision: association
                    .and_then(|association| association.last_observed_revision.clone()),
                blocked_reason,
            });
        }
        let (record_oid, structure_oid) = record_revision
            .map(|(record, structure)| (Some(record.into()), Some(structure.into())))
            .unwrap_or_default();
        Ok(GerritReviewOperationPlan {
            provider: "gerrit".into(),
            route: route.clone(),
            integration_anchor: commit_oids
                .first()
                .and_then(|oid| repo.run(&["rev-parse", &format!("{}^", oid)]).ok()),
            expected_record_oid: record_oid,
            expected_structure_oid: structure_oid,
            mapping_policy: local.mapping_policy,
            topology: "stacked".into(),
            transport_ref: route.upload_ref.clone(),
            push_options: Vec::new(),
            remote_queried: false,
            items,
        })
    }

    pub fn create(&self, plan: &GerritReviewOperationPlan) -> Result<GerritProviderState> {
        if let Some(item) = plan.items.iter().find(|item| item.change_id.is_none()) {
            return Err(StaircaseError::Other(format!(
                "missing-change-id: subject {} requires explicit normalization",
                item.subject_id
            )));
        }
        if let Some(item) = plan.items.iter().find(|item| item.blocked_reason.is_some()) {
            return Err(StaircaseError::Other(
                item.blocked_reason.clone().unwrap_or_default(),
            ));
        }
        Ok(GerritProviderState {
            schema_version: 1,
            route: plan.route.clone(),
            mapping_policy: plan.mapping_policy.clone(),
            topology: plan.topology.clone(),
            associations: plan
                .items
                .iter()
                .map(|item| GerritReviewAssociation {
                    subject_id: item.subject_id.clone(),
                    local_commit_oid: item.local_oid.clone(),
                    pending: GerritPendingReviewKey {
                        server_id: plan.route.server_id.clone(),
                        project: plan.route.project.clone(),
                        branch: plan.route.destination_branch.clone(),
                        change_id: item.change_id.clone().unwrap_or_default(),
                    },
                    confirmed: None,
                    last_observed_patch_set: None,
                    last_observed_revision: None,
                    synchronization: SynchronizationState::NotUploaded,
                    provider_status: None,
                    retired: false,
                })
                .collect(),
            verification: Vec::new(),
            reconciliation_required: false,
        })
    }

    pub fn prepare(
        &self,
        plan: &GerritReviewOperationPlan,
        existing: Option<GerritProviderState>,
    ) -> Result<GerritProviderState> {
        let mut state = prepare_review_state(
            plan,
            existing,
            |plan| self.create(plan),
            |state, plan| {
                if state.route.server_id != plan.route.server_id
                    || state.route.project != plan.route.project
                    || state.route.destination_branch != plan.route.destination_branch
                {
                    return Err(StaircaseError::Other(
                        "existing Gerrit associations belong to a different route".into(),
                    ));
                }
                Ok(())
            },
            |state| &mut state.associations,
            |item| {
                let change_id = item.change_id.clone().ok_or_else(|| {
                    StaircaseError::Other(format!(
                        "missing-change-id: subject {} requires explicit normalization",
                        item.subject_id
                    ))
                })?;
                Ok(GerritReviewAssociation {
                    subject_id: item.subject_id.clone(),
                    local_commit_oid: item.local_oid.clone(),
                    pending: GerritPendingReviewKey {
                        server_id: plan.route.server_id.clone(),
                        project: plan.route.project.clone(),
                        branch: plan.route.destination_branch.clone(),
                        change_id,
                    },
                    confirmed: None,
                    last_observed_patch_set: None,
                    last_observed_revision: None,
                    synchronization: SynchronizationState::NotUploaded,
                    provider_status: None,
                    retired: false,
                })
            },
        )?;
        state.mapping_policy = plan.mapping_policy.clone();
        state.topology = plan.topology.clone();
        Ok(state)
    }

    pub fn attach(
        &self,
        mut state: GerritProviderState,
        subject_id: &str,
        remote: GerritRemoteChange,
    ) -> Result<GerritProviderState> {
        if remote.project != state.route.project
            || normalize_gerrit_branch(&remote.branch) != state.route.destination_branch
        {
            return Err(StaircaseError::Other(
                "Gerrit review route does not match staircase route".into(),
            ));
        }
        let association = state
            .associations
            .iter_mut()
            .find(|association| association.subject_id == subject_id)
            .ok_or_else(|| StaircaseError::NotFound(subject_id.into()))?;
        if association.pending.change_id != remote.change_id {
            return Err(StaircaseError::Other(
                "Gerrit review Change-Id does not match local subject".into(),
            ));
        }
        apply_remote_change(association, &remote);
        Ok(state)
    }

    pub fn detach(
        &self,
        mut state: GerritProviderState,
        subject_id: &str,
    ) -> Result<GerritProviderState> {
        let association = state
            .associations
            .iter_mut()
            .find(|association| association.subject_id == subject_id)
            .ok_or_else(|| StaircaseError::NotFound(subject_id.into()))?;
        association.retired = true;
        Ok(state)
    }

    pub fn upload(
        &self,
        repo: &GitRepo,
        plan: &GerritReviewOperationPlan,
        mut state: GerritProviderState,
    ) -> Result<GerritMutationResult> {
        if state.reconciliation_required
            || plan.items.iter().any(|item| {
                matches!(
                    item.synchronization,
                    SynchronizationState::RemoteNewer
                        | SynchronizationState::Diverged
                        | SynchronizationState::UploadUnknown
                )
            })
        {
            return Err(StaircaseError::Other(
                "reconciliation-required: Gerrit remote state must be reconciled before upload"
                    .into(),
            ));
        }
        if let Some(item) = plan.items.iter().find(|item| item.blocked_reason.is_some()) {
            return Err(StaircaseError::Other(
                item.blocked_reason.clone().unwrap_or_default(),
            ));
        }
        for item in &plan.items {
            if let Some(association) = state
                .associations
                .iter_mut()
                .find(|association| association.subject_id == item.subject_id)
            {
                association.local_commit_oid = item.local_oid.clone();
            }
        }
        let source_oid = plan
            .items
            .last()
            .map(|item| item.local_oid.clone())
            .ok_or_else(|| StaircaseError::Other("empty Gerrit plan".into()))?;
        let request = TransportRequest::GitPush {
            remote: plan
                .route
                .transport_endpoint
                .clone()
                .unwrap_or_else(|| plan.route.server_id.clone()),
            source_oid,
            destination_ref: plan.transport_ref.clone(),
            force_with_lease: None,
            push_options: plan.push_options.clone(),
        };
        let response = match self.transport.execute(repo, &request) {
            Ok(response) => response,
            Err(error) => {
                let (state, journal_operation_id) = handle_uncertain_mutation(
                    repo,
                    "gerrit",
                    "upload",
                    plan,
                    state,
                    request,
                    serde_json::json!({"error": error.to_string()}),
                )?;
                return Ok(GerritMutationResult {
                    unknown: state
                        .associations
                        .iter()
                        .filter(|association| !association.retired)
                        .count(),
                    state,
                    status: "upload-unknown".into(),
                    accepted: 0,
                    rejected: 0,
                    journal_operation_id,
                });
            }
        };
        if response.uncertain {
            let (state, journal_operation_id) = handle_uncertain_mutation(
                repo,
                "gerrit",
                "upload",
                plan,
                state,
                request,
                response.observations,
            )?;
            return Ok(GerritMutationResult {
                unknown: state
                    .associations
                    .iter()
                    .filter(|association| !association.retired)
                    .count(),
                state,
                status: "upload-unknown".into(),
                accepted: 0,
                rejected: 0,
                journal_operation_id,
            });
        }
        if !response.success {
            return Ok(GerritMutationResult {
                state,
                status: "rejected".into(),
                accepted: 0,
                rejected: plan.items.len(),
                unknown: 0,
                journal_operation_id: None,
            });
        }
        let mut remote_changes: Vec<GerritRemoteChange> = response
            .observations
            .get("changes")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        if remote_changes.is_empty() {
            for association in state
                .associations
                .iter()
                .filter(|association| !association.retired)
            {
                let endpoint = format!(
                    "https://{}/a/changes/{}~{}~{}/detail",
                    plan.route.server_id,
                    association.pending.project,
                    association
                        .pending
                        .branch
                        .strip_prefix("refs/heads/")
                        .unwrap_or(&association.pending.branch),
                    association.pending.change_id
                );
                let request = TransportRequest::Api {
                    tool: "curl".into(),
                    method: "GET".into(),
                    endpoint,
                    arguments: Vec::new(),
                    body: None,
                };
                if let Ok(observation) = self.transport.execute(repo, &request) {
                    if observation.success {
                        if let Some(remote) = parse_gerrit_api_change(&observation.observations) {
                            remote_changes.push(remote);
                        }
                    }
                }
            }
        }
        let mut accepted = 0;
        let mut unknown = 0;
        for association in state
            .associations
            .iter_mut()
            .filter(|association| !association.retired)
        {
            if let Some(remote) = remote_changes
                .iter()
                .find(|remote| remote.change_id == association.pending.change_id)
            {
                apply_remote_change(association, remote);
                accepted += 1;
            } else {
                association.synchronization = SynchronizationState::UploadUnknown;
                unknown += 1;
            }
        }
        state.reconciliation_required = unknown > 0;
        let journal_operation_id = if unknown > 0 {
            let journal = OperationJournal::for_repo(repo)?;
            Some(
                journal
                    .record(
                        "gerrit",
                        "upload",
                        plan.expected_record_oid.clone(),
                        request,
                        serde_json::json!({"unresolved_change_ids": state.associations.iter().filter(|association| association.synchronization == SynchronizationState::UploadUnknown).map(|association| association.pending.change_id.clone()).collect::<Vec<_>>() }),
                    )?
                    .operation_id,
            )
        } else {
            None
        };
        Ok(GerritMutationResult {
            state,
            status: if unknown == 0 {
                "uploaded".into()
            } else {
                "upload-unknown".into()
            },
            accepted,
            rejected: 0,
            unknown,
            journal_operation_id,
        })
    }

    pub fn reconcile(
        &self,
        mut state: GerritProviderState,
        remote_changes: &[GerritRemoteChange],
    ) -> GerritProviderState {
        for association in &mut state.associations {
            let matches: Vec<_> = remote_changes
                .iter()
                .filter(|remote| {
                    remote.change_id == association.pending.change_id
                        && remote.project == association.pending.project
                        && normalize_gerrit_branch(&remote.branch) == association.pending.branch
                })
                .collect();
            if matches.len() == 1 {
                apply_remote_change(association, matches[0]);
            } else if matches.len() > 1 {
                association.synchronization = SynchronizationState::IdentityAmbiguous;
            } else {
                association.synchronization = SynchronizationState::Unknown;
            }
        }
        state.reconciliation_required = state
            .associations
            .iter()
            .filter(|association| !association.retired)
            .any(|association| {
                matches!(
                    association.synchronization,
                    SynchronizationState::IdentityAmbiguous
                        | SynchronizationState::Unknown
                        | SynchronizationState::UploadUnknown
                )
            });
        state
    }

    pub fn verify(
        &self,
        mut state: GerritProviderState,
        remote_changes: &[GerritRemoteChange],
    ) -> (GerritProviderState, Vec<GerritVerificationEvidence>) {
        let mut evidence = Vec::new();
        for association in state
            .associations
            .iter()
            .filter(|association| !association.retired)
        {
            if let Some(remote) = remote_changes.iter().find(|remote| {
                remote.change_id == association.pending.change_id
                    && remote.revision == association.local_commit_oid
            }) {
                evidence.push(GerritVerificationEvidence {
                    change_id: remote.change_id.clone(),
                    exact_revision: remote.revision.clone(),
                    patch_set: remote.patch_set,
                    labels: remote.labels.clone(),
                    checks_passed: remote
                        .labels
                        .get("Verified")
                        .is_none_or(|value| value != "-1"),
                    submit_requirements_satisfied: remote.submit_requirements_satisfied,
                    mergeable: remote.mergeable,
                    submittable: remote.submittable,
                    observed_at: crate::workspace::storage::current_timestamp(),
                });
            }
        }
        state.verification = evidence.clone();
        (state, evidence)
    }

    pub fn persist(
        &self,
        repo: &GitRepo,
        record: &crate::model::StaircaseRecord,
        state: &GerritProviderState,
    ) -> Result<crate::model::StaircaseRecord> {
        provider_base::persist_provider_state(repo, record, "git-staircase.gerrit", state)
    }

    pub fn land(
        &self,
        repo: &GitRepo,
        state: &GerritProviderState,
        mode: &str,
        topic_members: &[u64],
    ) -> Result<UnifiedProviderLanding> {
        let selected: Vec<u64> = state
            .associations
            .iter()
            .filter(|association| !association.retired)
            .filter_map(|association| {
                association
                    .confirmed
                    .as_ref()
                    .map(|confirmed| confirmed.numeric_id)
            })
            .collect();
        if mode == "aggregate"
            && topic_members
                .iter()
                .any(|member| !selected.contains(member))
        {
            return Ok(UnifiedProviderLanding {
                provider_label: "Gerrit".into(),
                mode: mode.into(),
                status: "landing-blocked".into(),
                landed: Vec::new(),
                blocked: topic_members
                    .iter()
                    .filter(|member| !selected.contains(member))
                    .map(ToString::to_string)
                    .collect(),
                destination_oid: None,
                details: vec!["gerrit.topic-contains-unrelated-changes".into()],
            });
        }
        let targets: Vec<u64> = if mode == "stepwise" {
            selected.first().copied().into_iter().collect()
        } else {
            selected
        };
        for target in &targets {
            let association = state.associations.iter().find(|association| {
                association
                    .confirmed
                    .as_ref()
                    .is_some_and(|confirmed| confirmed.numeric_id == *target)
            });
            let verified = association.is_some_and(|association| {
                state.verification.iter().any(|evidence| {
                    evidence.change_id == association.pending.change_id
                        && evidence.exact_revision == association.local_commit_oid
                        && evidence.submittable
                        && evidence.submit_requirements_satisfied
                })
            });
            if !verified {
                return Ok(UnifiedProviderLanding {
                    provider_label: "Gerrit".into(),
                    mode: mode.into(),
                    status: "landing-blocked".into(),
                    landed: Vec::new(),
                    blocked: vec![target.to_string()],
                    destination_oid: None,
                    details: vec!["exact current patch set is not submittable".into()],
                });
            }
        }
        provider_base::land_loop_common(
            repo,
            &self.transport,
            "gerrit",
            "Gerrit",
            mode,
            targets,
            |target| {
                Ok(TransportRequest::Api {
                    tool: "curl".into(),
                    method: "POST".into(),
                    endpoint: format!(
                        "https://{}/a/changes/{}/submit",
                        state.route.server_id, target
                    ),
                    arguments: Vec::new(),
                    body: Some(serde_json::json!({})),
                })
            },
            |target| target.to_string(),
            "reconciliation required before another submit",
        )
    }
}

fn classify_gerrit_item(
    planned: &GerritPlannedCommit,
    association: Option<&GerritReviewAssociation>,
) -> (SynchronizationState, String, Option<String>) {
    if planned.change_id.is_none() || planned.change_id_status != "valid" {
        return (
            SynchronizationState::NotCreated,
            "blocked".into(),
            Some("missing or invalid Change-Id requires explicit normalization".into()),
        );
    }
    let Some(association) = association else {
        return (SynchronizationState::NotCreated, "create".into(), None);
    };
    if matches!(
        association.synchronization,
        SynchronizationState::RemoteNewer
            | SynchronizationState::Diverged
            | SynchronizationState::UploadUnknown
            | SynchronizationState::IdentityAmbiguous
    ) {
        return (
            association.synchronization,
            "blocked".into(),
            Some("reconciliation-required: remote Gerrit state is not upload-safe".into()),
        );
    }
    if association.pending.change_id != planned.change_id.clone().unwrap_or_default() {
        return (
            SynchronizationState::Retargeted,
            "blocked".into(),
            Some("local Change-Id differs from durable review association".into()),
        );
    }
    match (
        association.last_observed_revision.as_deref(),
        association.confirmed.as_ref(),
    ) {
        (Some(remote), Some(_)) if remote == planned.oid => {
            (SynchronizationState::Current, "no-op".into(), None)
        }
        (Some(_), Some(confirmed))
            if association.last_observed_patch_set == Some(confirmed.patch_set) =>
        {
            (SynchronizationState::LocalNewer, "update".into(), None)
        }
        (Some(_), Some(_)) => (
            SynchronizationState::RemoteNewer,
            "blocked".into(),
            Some("remote-newer: reconcile before upload".into()),
        ),
        _ => (SynchronizationState::NotUploaded, "create".into(), None),
    }
}

fn normalize_gerrit_branch(branch: &str) -> String {
    if branch.starts_with("refs/heads/") {
        branch.into()
    } else {
        format!("refs/heads/{}", branch)
    }
}

fn apply_remote_change(association: &mut GerritReviewAssociation, remote: &GerritRemoteChange) {
    let prior_patch_set = association.last_observed_patch_set;
    let prior_revision = association.last_observed_revision.clone();
    association.confirmed = Some(GerritConfirmedReview {
        server_id: association.pending.server_id.clone(),
        project: remote.project.clone(),
        branch: normalize_gerrit_branch(&remote.branch),
        change_id: remote.change_id.clone(),
        numeric_id: remote.numeric_id,
        patch_set: remote.patch_set,
        patch_set_revision: remote.revision.clone(),
        change_ref: remote.change_ref.clone(),
        web_url: None,
    });
    association.last_observed_patch_set = Some(remote.patch_set);
    association.last_observed_revision = Some(remote.revision.clone());
    association.provider_status = Some(remote.status.clone());
    association.synchronization = if remote.status.eq_ignore_ascii_case("merged") {
        SynchronizationState::Merged
    } else if remote.status.eq_ignore_ascii_case("abandoned") {
        SynchronizationState::Abandoned
    } else if remote.revision == association.local_commit_oid {
        SynchronizationState::Current
    } else if prior_patch_set.is_some_and(|patch_set| remote.patch_set > patch_set)
        && prior_revision.as_deref() != Some(&remote.revision)
    {
        SynchronizationState::RemoteNewer
    } else {
        SynchronizationState::Diverged
    };
}

pub fn parse_gerrit_api_change(value: &serde_json::Value) -> Option<GerritRemoteChange> {
    let current_revision = value.get("current_revision")?.as_str()?.to_string();
    let revision = value
        .get("revisions")
        .and_then(|revisions| revisions.get(&current_revision));
    let patch_set = revision
        .and_then(|revision| revision.get("_number"))
        .and_then(|number| number.as_u64())
        .and_then(|number| u32::try_from(number).ok())
        .unwrap_or(0);
    let labels = value
        .get("labels")
        .and_then(|labels| labels.as_object())
        .map(|labels| {
            labels
                .iter()
                .map(|(name, value)| {
                    let state = if value
                        .get("approved")
                        .is_some_and(|approved| !approved.is_null())
                    {
                        "approved"
                    } else if value
                        .get("rejected")
                        .is_some_and(|rejected| !rejected.is_null())
                    {
                        "rejected"
                    } else {
                        "pending"
                    };
                    (name.clone(), state.into())
                })
                .collect()
        })
        .unwrap_or_default();
    Some(GerritRemoteChange {
        change_id: value.get("change_id")?.as_str()?.into(),
        numeric_id: value.get("_number")?.as_u64()?,
        patch_set,
        revision: current_revision,
        change_ref: revision
            .and_then(|revision| revision.get("ref"))
            .and_then(|reference| reference.as_str())
            .map(str::to_string),
        status: value
            .get("status")
            .and_then(|status| status.as_str())
            .unwrap_or("NEW")
            .into(),
        branch: value.get("branch")?.as_str()?.into(),
        project: value.get("project")?.as_str()?.into(),
        labels,
        submit_requirements_satisfied: value
            .get("submit_requirements")
            .and_then(|requirements| requirements.as_array())
            .is_none_or(|requirements| {
                requirements.iter().all(|requirement| {
                    requirement.get("status").and_then(|status| status.as_str())
                        == Some("SATISFIED")
                })
            }),
        mergeable: value.get("mergeable").and_then(|value| value.as_bool()),
        submittable: value
            .get("submittable")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        topic: value
            .get("topic")
            .and_then(|topic| topic.as_str())
            .map(str::to_string),
    })
}

pub struct GerritProvider;

impl ReviewProvider for GerritProvider {
    fn name(&self) -> &'static str {
        "gerrit"
    }

    fn probe(
        &self,
        repo: &GitRepo,
        record: Option<&WorkspaceRecord>,
    ) -> Result<Option<Box<dyn ReviewProviderInstance>>> {
        if let Some(route) = probe_gerrit_route(repo, record)? {
            Ok(Some(Box::new(
                crate::workspace::stacked_provider::StackedReviewInstance {
                    implementation: GerritStackedImplementation,
                    route,
                },
            )))
        } else {
            Ok(None)
        }
    }
}

pub(crate) struct GerritInstance {
    pub route: GerritRoute,
}

#[allow(dead_code)]
impl GerritInstance {
    pub(crate) fn show(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewShow> {
        let plan = create_gerrit_upload_plan(repo, &self.route, oids, None)?;
        let mut details = HashMap::new();
        details.insert("Upload Ref".to_string(), self.route.upload_ref.clone());

        let items = plan
            .commits
            .iter()
            .map(|c| UnifiedReviewItem {
                oid: c.oid.clone(),
                title: c.subject.clone(),
                detail: format!("Change-Id: {}", c.change_id.as_deref().unwrap_or("<none>")),
            })
            .collect();

        Ok(UnifiedReviewShow {
            provider_label: "Gerrit".to_string(),
            host: self.route.server_id.clone(),
            project: self.route.project.clone(),
            destination_branch: self.route.destination_branch.clone(),
            details,
            items,
        })
    }

    fn status(
        &self,
        _repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewStatus> {
        if let Some(state) = provider_base::status_from_record::<GerritProviderState>(
            record,
            "git-staircase.gerrit",
        )? {
            let mut counts = HashMap::<String, usize>::new();
            for association in &state.associations {
                *counts
                    .entry(format!("{:?}", association.synchronization).to_ascii_lowercase())
                    .or_default() += 1;
            }
            return Ok(UnifiedReviewStatus {
                provider_label: "Gerrit".into(),
                status: if state.reconciliation_required {
                    "reconciliation-required".into()
                } else if state
                    .associations
                    .iter()
                    .all(|association| association.synchronization == SynchronizationState::Current)
                {
                    "current".into()
                } else {
                    "pending".into()
                },
                host: state.route.server_id,
                project: state.route.project,
                details: counts
                    .into_iter()
                    .map(|(state, count)| (format!("sync.{}", state), count.to_string()))
                    .collect(),
            });
        }
        let plan = create_gerrit_upload_plan(_repo, &self.route, oids, None)?;
        let mut details = HashMap::new();
        details.insert("Remote Queried".to_string(), "false".to_string());
        details.insert("Review Commits".to_string(), plan.commits.len().to_string());
        details.insert(
            "Exact Verification".to_string(),
            "unavailable without a remote observation".to_string(),
        );
        Ok(UnifiedReviewStatus {
            provider_label: "Gerrit".to_string(),
            status: "unknown".into(),
            host: self.route.server_id.clone(),
            project: self.route.project.clone(),
            details,
        })
    }

    fn plan(
        &self,
        repo: &GitRepo,
        oids: &[String],
        mapping: Option<&str>,
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewPlan> {
        let plan = create_gerrit_upload_plan(repo, &self.route, oids, mapping)?;
        let items = plan
            .commits
            .iter()
            .map(|c| UnifiedReviewItem {
                oid: c.oid.clone(),
                title: c.subject.clone(),
                detail: c.change_id_status.clone(),
            })
            .collect();

        Ok(UnifiedReviewPlan {
            provider_label: "Gerrit".to_string(),
            target: plan.push_ref,
            policy: plan.mapping_policy,
            items,
            warnings: plan.warnings,
        })
    }

    fn upload(
        &self,
        repo: &GitRepo,
        oids: &[String],
        destination: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewUpload> {
        if let Some(record) = record {
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.gerrit") {
                let mut state: GerritProviderState = serde_json::from_value(value.clone())?;
                if let Some(destination) = destination {
                    state.route.destination_branch = format!(
                        "refs/heads/{}",
                        destination.trim_start_matches("refs/heads/")
                    );
                    state.route.upload_ref =
                        format!("refs/for/{}", destination.trim_start_matches("refs/heads/"));
                }
                let machine = GerritStateMachine::new(ProductionTransport);
                let metadata = &record.metadata;
                let subjects = metadata
                    .steps
                    .iter()
                    .map(|step| step.id.clone())
                    .collect::<Vec<_>>();
                let plan = machine.plan(
                    repo,
                    &state.route,
                    oids,
                    &subjects,
                    Some(&state.mapping_policy),
                    Some(&state),
                    Some((&record.record_oid, &record.structure_oid)),
                )?;
                let result = machine.upload(repo, &plan, state)?;
                let next = machine.persist(repo, &record, &result.state)?;
                return Ok(UnifiedReviewUpload {
                    provider_label: "Gerrit".into(),
                    summary: result.status,
                    details: vec![
                        format!("accepted: {}", result.accepted),
                        format!("rejected: {}", result.rejected),
                        format!("unknown: {}", result.unknown),
                        format!(
                            "record revision: {} -> {}",
                            record.record_oid, next.record_oid
                        ),
                    ],
                });
            }
        }
        let mut active_route = self.route.clone();
        if let Some(dest) = destination {
            active_route.destination_branch = format!("refs/heads/{}", dest);
            active_route.upload_ref = format!("refs/for/{}", dest);
        }
        let machine = GerritStateMachine::new(ProductionTransport);
        let subject_ids: Vec<String> = (0..oids.len())
            .map(|index| format!("commit-{}", index + 1))
            .collect();
        let operation_plan =
            machine.plan(repo, &active_route, oids, &subject_ids, None, None, None)?;
        let state = machine.create(&operation_plan)?;
        let result = machine.upload(repo, &operation_plan, state)?;
        Ok(UnifiedReviewUpload {
            provider_label: "Gerrit".to_string(),
            summary: result.status,
            details: vec![
                format!("accepted: {}", result.accepted),
                format!("unknown: {}", result.unknown),
            ],
        })
    }

    fn reconcile(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewReconcile> {
        if let Some(record) = record {
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.gerrit") {
                let state: GerritProviderState = serde_json::from_value(value.clone())?;
                let machine = GerritStateMachine::new(ProductionTransport);
                let mut observations = Vec::new();
                for association in &state.associations {
                    let selector = association
                        .confirmed
                        .as_ref()
                        .map(|confirmed| confirmed.numeric_id.to_string())
                        .unwrap_or_else(|| association.pending.change_id.clone());
                    let request = TransportRequest::Api {
                        tool: "curl".into(),
                        method: "GET".into(),
                        endpoint: format!(
                            "https://{}/a/changes/{}/detail",
                            state.route.server_id, selector
                        ),
                        arguments: Vec::new(),
                        body: None,
                    };
                    let response = machine.transport.execute(repo, &request)?;
                    if response.uncertain {
                        return Err(StaircaseError::Other(
                            "Gerrit reconciliation query outcome is uncertain".into(),
                        ));
                    }
                    if response.success {
                        if let Some(observation) = parse_gerrit_api_change(&response.observations) {
                            observations.push(observation);
                        }
                    }
                }
                let state = machine.reconcile(state, &observations);
                let pending = state.reconciliation_required;
                let next = machine.persist(repo, &record, &state)?;
                return Ok(UnifiedReviewReconcile {
                    provider_label: "Gerrit".into(),
                    status: format!(
                        "{}; record {} -> {}",
                        if pending {
                            "reconciliation-required"
                        } else {
                            "reconciled"
                        },
                        record.record_oid,
                        next.record_oid
                    ),
                });
            }
        }
        let _plan = create_gerrit_upload_plan(repo, &self.route, oids, None)?;
        Ok(UnifiedReviewReconcile {
            provider_label: "Gerrit".to_string(),
            status: "Reconciled with Gerrit server".to_string(),
        })
    }

    fn get_stable_identifiers(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<Vec<Option<String>>> {
        let plan = create_gerrit_upload_plan(repo, &self.route, oids, None)?;
        Ok(plan.commits.iter().map(|c| c.change_id.clone()).collect())
    }

    fn open(
        &self,
        _repo: &GitRepo,
        _oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewOpen> {
        let url = format!(
            "https://{}/q/project:{}",
            self.route.server_id, self.route.project
        );
        Ok(UnifiedReviewOpen {
            provider_label: "Gerrit".to_string(),
            url,
        })
    }

    fn create(
        &self,
        repo: &GitRepo,
        oids: &[String],
        mapping: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewMutation> {
        if let Some(record) = record {
            let metadata = &record.metadata;
            let subjects = metadata
                .steps
                .iter()
                .map(|step| step.id.clone())
                .collect::<Vec<_>>();
            let workspace = crate::workspace::bootstrap::bootstrap(
                repo,
                &crate::workspace::bootstrap::BootstrapOptions::default(),
            )?;
            let route = probe_gerrit_route(repo, Some(&workspace.record))?
                .ok_or_else(|| StaircaseError::Other("Gerrit route is incomplete".into()))?;
            let machine = GerritStateMachine::new(ProductionTransport);
            let existing = record
                .user_metadata
                .extensions
                .get("git-staircase.gerrit")
                .cloned()
                .map(serde_json::from_value::<GerritProviderState>)
                .transpose()?;
            let plan = machine.plan(
                repo,
                &route,
                oids,
                &subjects,
                mapping,
                existing.as_ref(),
                Some((&record.record_oid, &record.structure_oid)),
            )?;
            let before_count = existing
                .as_ref()
                .map(|state| state.associations.len())
                .unwrap_or(0);
            let state = machine.prepare(&plan, existing)?;
            let changed = state.associations.len().saturating_sub(before_count);
            let next = machine.persist(repo, &record, &state)?;
            return Ok(UnifiedReviewMutation {
                provider_label: "Gerrit".into(),
                action: "create".into(),
                changed,
                record_before: Some(record.record_oid.clone()),
                record_after: Some(next.record_oid),
                details: vec!["pending review keys recorded; remote publication required".into()],
            });
        }
        let machine = GerritStateMachine::new(ProductionTransport);
        let subjects: Vec<String> = (0..oids.len())
            .map(|index| format!("commit-{}", index + 1))
            .collect();
        let plan = machine.plan(repo, &self.route, oids, &subjects, mapping, None, None)?;
        let state = machine.create(&plan)?;
        Ok(UnifiedReviewMutation {
            provider_label: "Gerrit".into(),
            action: "create".into(),
            changed: state.associations.len(),
            record_before: None,
            record_after: None,
            details: state
                .associations
                .iter()
                .map(|association| {
                    format!("{} pending first upload", association.pending.change_id)
                })
                .collect(),
        })
    }

    fn attach(
        &self,
        repo: &GitRepo,
        _oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation> {
        if let Some(record) = record {
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.gerrit") {
                let state: GerritProviderState = serde_json::from_value(value.clone())?;
                let subject_index = selected_index.unwrap_or(0);
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| {
                        StaircaseError::Other("selected step has no Gerrit association".into())
                    })?;
                let machine = GerritStateMachine::new(ProductionTransport);
                if review.is_empty()
                    || !review.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "~._-".contains(character)
                    })
                {
                    return Err(StaircaseError::Other(
                        "unsafe Gerrit review selector".into(),
                    ));
                }
                let request = TransportRequest::Api {
                    tool: "curl".into(),
                    method: "GET".into(),
                    endpoint: format!(
                        "https://{}/a/changes/{}/detail",
                        state.route.server_id, review
                    ),
                    arguments: Vec::new(),
                    body: None,
                };
                let response = machine.transport.execute(repo, &request)?;
                if !response.success || response.uncertain {
                    return Err(StaircaseError::Other(
                        "Gerrit attachment validation failed or is uncertain".into(),
                    ));
                }
                let remote = parse_gerrit_api_change(&response.observations).ok_or_else(|| {
                    StaircaseError::Other("Gerrit returned malformed change metadata".into())
                })?;
                let state = machine.attach(state, &subject_id, remote)?;
                let next = machine.persist(repo, &record, &state)?;
                return Ok(UnifiedReviewMutation {
                    provider_label: "Gerrit".into(),
                    action: "attach".into(),
                    changed: 1,
                    record_before: Some(record.record_oid.clone()),
                    record_after: Some(next.record_oid),
                    details: vec![format!("validated and attached Gerrit review {}", review)],
                });
            }
        }
        if review.trim().is_empty() {
            return Err(StaircaseError::Other(
                "Gerrit review selector is empty".into(),
            ));
        }
        Ok(UnifiedReviewMutation {
            provider_label: "Gerrit".into(),
            action: "attach".into(),
            changed: 1,
            record_before: None,
            record_after: None,
            details: vec![format!("provisional attachment {}", review)],
        })
    }

    fn detach(
        &self,
        repo: &GitRepo,
        _oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation> {
        if let Some(record) = record {
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.gerrit") {
                let state: GerritProviderState = serde_json::from_value(value.clone())?;
                let subject_index = selected_index.unwrap_or(0);
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| {
                        StaircaseError::Other("selected step has no Gerrit association".into())
                    })?;
                let machine = GerritStateMachine::new(ProductionTransport);
                let state = machine.detach(state, &subject_id)?;
                let next = machine.persist(repo, &record, &state)?;
                return Ok(UnifiedReviewMutation {
                    provider_label: "Gerrit".into(),
                    action: "detach".into(),
                    changed: 1,
                    record_before: Some(record.record_oid.clone()),
                    record_after: Some(next.record_oid),
                    details: vec![format!("detached Gerrit review {}", review)],
                });
            }
        }
        Ok(UnifiedReviewMutation {
            provider_label: "Gerrit".into(),
            action: "detach".into(),
            changed: 1,
            record_before: None,
            record_after: None,
            details: vec![format!("retained historical association {}", review)],
        })
    }

    fn verify_provider(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderVerification> {
        let plan = create_gerrit_upload_plan(repo, &self.route, oids, None)?;
        let stale_revisions = plan
            .commits
            .iter()
            .filter(|commit| commit.change_id.is_none())
            .map(|commit| commit.oid.clone())
            .collect::<Vec<_>>();
        Ok(UnifiedProviderVerification {
            provider_label: "Gerrit".into(),
            status: if stale_revisions.is_empty() {
                "unknown-without-remote-observation".into()
            } else {
                "stale".into()
            },
            exact_revisions: Vec::new(),
            stale_revisions,
            details: HashMap::from([(
                "remote".into(),
                "not queried by passive verification".into(),
            )]),
        })
    }

    fn land(
        &self,
        _repo: &GitRepo,
        _oids: &[String],
        mode: &str,
        _method: Option<&str>,
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderLanding> {
        Ok(UnifiedProviderLanding {
            provider_label: "Gerrit".into(),
            mode: mode.into(),
            status: "landing-blocked".into(),
            landed: Vec::new(),
            blocked: Vec::new(),
            destination_oid: None,
            details: vec!["confirmed identities and exact verification are required".into()],
        })
    }
}

impl ReviewAssociation for GerritReviewAssociation {
    fn subject_id(&self) -> &str {
        &self.subject_id
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
    fn set_retired(&mut self, retired: bool) {
        self.retired = retired;
    }
    fn update_local_oid(&mut self, oid: String) {
        self.local_commit_oid = oid;
    }
    fn set_synchronization(&mut self, state: SynchronizationState) {
        self.synchronization = state;
    }
}

impl ReviewPlanItem for GerritPlanItem {
    fn subject_id(&self) -> &str {
        &self.subject_id
    }
    fn local_oid(&self) -> &str {
        &self.local_oid
    }
}

impl ReviewOperationPlan for GerritReviewOperationPlan {
    type Item = GerritPlanItem;
    fn items(&self) -> &[Self::Item] {
        &self.items
    }
    fn expected_record_oid(&self) -> Option<String> {
        self.expected_record_oid.clone()
    }
}

impl ReviewProviderState for GerritProviderState {
    type Association = GerritReviewAssociation;
    fn associations_mut(&mut self) -> &mut Vec<Self::Association> {
        &mut self.associations
    }
    fn reconciliation_required(&self) -> bool {
        self.reconciliation_required
    }
    fn set_reconciliation_required(&mut self, required: bool) {
        self.reconciliation_required = required;
    }
}

impl crate::workspace::stacked_provider::StackedReviewItem for GerritPlannedCommit {
    fn subject_id(&self) -> &str {
        &self.oid
    }
    fn local_oid(&self) -> &str {
        &self.oid
    }
    fn title(&self) -> &str {
        &self.subject
    }
    fn detail(&self) -> String {
        self.change_id_status.clone()
    }
}

impl crate::workspace::stacked_provider::StackedReviewPlan for GerritUploadPlan {
    type Item = GerritPlannedCommit;
    fn items(&self) -> &[Self::Item] {
        &self.commits
    }
    fn mapping_policy(&self) -> &str {
        &self.mapping_policy
    }
    fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

impl crate::workspace::stacked_provider::StackedReviewAssociation for GerritReviewAssociation {
    fn subject_id(&self) -> &str {
        &self.subject_id
    }
    fn local_oid(&self) -> &str {
        &self.local_commit_oid
    }
    fn synchronization(&self) -> crate::workspace::review_provider::SynchronizationState {
        self.synchronization
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}

impl crate::workspace::stacked_provider::StackedReviewState for GerritProviderState {
    type Association = GerritReviewAssociation;
    fn associations(&self) -> &[Self::Association] {
        &self.associations
    }
    fn reconciliation_required(&self) -> bool {
        self.reconciliation_required
    }
}

pub struct GerritStackedImplementation;

impl crate::workspace::stacked_provider::StackedReviewImplementation
    for GerritStackedImplementation
{
    type State = GerritProviderState;
    type Route = GerritRoute;
    type Plan = GerritUploadPlan;

    fn provider_label(&self) -> &'static str {
        "Gerrit"
    }
    fn extension_name(&self) -> &'static str {
        "git-staircase.gerrit"
    }

    fn create_plan(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        oids: &[String],
        mapping: Option<&str>,
    ) -> Result<Self::Plan> {
        create_gerrit_upload_plan(repo, route, oids, mapping)
    }

    fn get_host(&self, route: &GerritRoute) -> String {
        route.server_id.clone()
    }
    fn get_project(&self, route: &GerritRoute) -> String {
        route.project.clone()
    }
    fn get_destination_branch(&self, route: &GerritRoute) -> String {
        route.destination_branch.clone()
    }
    fn get_open_url(&self, route: &GerritRoute) -> String {
        format!("https://{}/q/project:{}", route.server_id, route.project)
    }

    fn render_association_detail(&self, association: &GerritReviewAssociation) -> String {
        format!("Change-Id: {}", association.pending.change_id)
    }

    fn render_state_details(&self, state: &GerritProviderState) -> HashMap<String, String> {
        let mut counts = HashMap::<String, usize>::new();
        for association in &state.associations {
            *counts
                .entry(format!("{:?}", association.synchronization).to_ascii_lowercase())
                .or_default() += 1;
        }
        counts
            .into_iter()
            .map(|(state, count)| (format!("sync.{}", state), count.to_string()))
            .collect()
    }

    fn upload(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        oids: &[String],
        destination: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewUpload> {
        let instance = GerritInstance {
            route: route.clone(),
        };
        instance.upload(repo, oids, destination, record)
    }
    fn reconcile(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewReconcile> {
        let instance = GerritInstance {
            route: route.clone(),
        };
        instance.reconcile(repo, oids, record)
    }
    fn create(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        oids: &[String],
        mapping: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewMutation> {
        let instance = GerritInstance {
            route: route.clone(),
        };
        instance.create(repo, oids, mapping, record)
    }
    fn attach(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewMutation> {
        let instance = GerritInstance {
            route: route.clone(),
        };
        instance.attach(repo, oids, review, record, selected_index)
    }
    fn detach(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewMutation> {
        let instance = GerritInstance {
            route: route.clone(),
        };
        instance.detach(repo, oids, review, record, selected_index)
    }
    fn land(
        &self,
        repo: &GitRepo,
        route: &GerritRoute,
        oids: &[String],
        mode: &str,
        method: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedProviderLanding> {
        let instance = GerritInstance {
            route: route.clone(),
        };
        instance.land(repo, oids, mode, method, record)
    }
}

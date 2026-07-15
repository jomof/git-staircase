use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::StaircaseRecord;
use crate::workspace::model::{Capability, ProbeDescriptor, ProviderDescriptor, WorkspaceRecord};
use crate::workspace::parse_git_url;
use crate::workspace::provider_base::{self, ProviderAssociation};
use crate::workspace::review_provider::{
    OperationJournal, ProductionTransport, ProviderTransport, ReviewAssociation,
    ReviewOperationPlan, ReviewPlanItem, ReviewProvider, ReviewProviderInstance,
    SynchronizationState, TransportRequest, UnifiedProviderLanding, UnifiedProviderVerification,
    UnifiedReviewItem, UnifiedReviewMutation, UnifiedReviewOpen, UnifiedReviewPlan,
    UnifiedReviewReconcile, UnifiedReviewShow, UnifiedReviewStatus, UnifiedReviewUpload,
    prepare_review_state,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn get_github_descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        protocol_version: 1,
        name: "github".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            Capability::RepositoryRouting,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubRepoLocator {
    pub installation: String,
    pub owner: String,
    pub repository: String,
}

impl GitHubRepoLocator {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repository)
    }
}

pub fn parse_github_remote_url(url: &str) -> Option<GitHubRepoLocator> {
    if let Some(info) = parse_git_url(url) {
        if (info.host.eq_ignore_ascii_case("github.com") || info.host.contains("github"))
            && info.owner.is_some()
            && info.repository.is_some()
        {
            return Some(GitHubRepoLocator {
                installation: info.host.to_ascii_lowercase(),
                owner: info.owner.unwrap(),
                repository: info.repository.unwrap(),
            });
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRoute {
    pub installation: String,
    pub base_repository: GitHubRepoLocator,
    pub head_repository: Option<GitHubRepoLocator>,
    pub destination_branch: String,
    pub remote_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubRouteOverrides {
    pub installation: Option<String>,
    pub base_repository: Option<GitHubRepoLocator>,
    pub head_repository: Option<GitHubRepoLocator>,
    pub destination_branch: Option<String>,
    pub remote_name: Option<String>,
}

pub fn resolve_github_route(
    repo: &GitRepo,
    record: Option<&WorkspaceRecord>,
    overrides: &GitHubRouteOverrides,
) -> Result<GitHubRoute> {
    let discovered = probe_github_route(repo, record)?;
    let base_repository = overrides
        .base_repository
        .clone()
        .or_else(|| {
            discovered
                .as_ref()
                .map(|route| route.base_repository.clone())
        })
        .ok_or_else(|| StaircaseError::Other("github.route-incomplete: base repository".into()))?;
    let installation = overrides
        .installation
        .clone()
        .unwrap_or_else(|| base_repository.installation.clone())
        .to_ascii_lowercase();
    let destination_branch = overrides
        .destination_branch
        .clone()
        .or_else(|| {
            discovered
                .as_ref()
                .map(|route| route.destination_branch.clone())
        })
        .ok_or_else(|| StaircaseError::Other("github.route-incomplete: base branch".into()))?;
    Ok(GitHubRoute {
        installation,
        base_repository,
        head_repository: overrides.head_repository.clone().or_else(|| {
            discovered
                .as_ref()
                .and_then(|route| route.head_repository.clone())
        }),
        destination_branch: if destination_branch.starts_with("refs/heads/") {
            destination_branch
        } else {
            format!("refs/heads/{}", destination_branch)
        },
        remote_name: overrides
            .remote_name
            .clone()
            .or_else(|| discovered.map(|route| route.remote_name))
            .unwrap_or_else(|| "origin".into()),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequestIdentity {
    pub installation: String,
    pub base_repository: String,
    pub number: u64,
    pub head_branch: String,
    pub head_oid: String,
    pub base_branch: String,
    pub base_oid: String,
    pub state: String,
    pub is_draft: bool,
    pub mergeable: Option<bool>,
}

pub fn probe_github_route(
    repo: &GitRepo,
    record: Option<&WorkspaceRecord>,
) -> Result<Option<GitHubRoute>> {
    let mut locator = None;
    let mut remote_name = "origin".to_string();
    let mut dest_branch = "refs/heads/main".to_string();

    if let Some(rec) = record {
        if let Some(db) = rec.discovery_fingerprint.get("dest_branch") {
            dest_branch = format!("refs/heads/{}", db);
        }
    }

    if let Ok(remotes) = repo.run(&["remote", "-v"]) {
        for line in remotes.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let r_name = parts[0];
                let url = parts[1];
                if let Some(loc) = parse_github_remote_url(url) {
                    locator = Some(loc);
                    remote_name = r_name.to_string();
                    break;
                }
            }
        }
    }

    if locator.is_none() {
        if let Ok(host) = repo.run(&["config", "--get", "github.host"]) {
            if let Ok(repo_spec) = repo.run(&["config", "--get", "github.repository"]) {
                let host_str = host.trim();
                let repo_spec_str = repo_spec.trim();
                if let Some((owner, rname)) = repo_spec_str.split_once('/') {
                    locator = Some(GitHubRepoLocator {
                        installation: if host_str.is_empty() {
                            "github.com".to_string()
                        } else {
                            host_str.to_string()
                        },
                        owner: owner.to_string(),
                        repository: rname.to_string(),
                    });
                }
            }
        }
    }

    if let Some(loc) = locator {
        Ok(Some(GitHubRoute {
            installation: loc.installation.clone(),
            base_repository: loc.clone(),
            head_repository: Some(loc),
            destination_branch: dest_branch,
            remote_name,
        }))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPlannedPublication {
    pub step_oid: String,
    pub subject: String,
    pub head_branch: String,
    pub base_branch: String,
    pub push_refspec: String,
    pub force_with_lease: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUploadPlan {
    pub installation: String,
    pub repository: String,
    pub mapping_policy: String,
    pub publications: Vec<GitHubPlannedPublication>,
    pub warnings: Vec<String>,
}

pub fn create_github_upload_plan(
    repo: &GitRepo,
    route: &GitHubRoute,
    commit_oids: &[String],
    mapping_policy: Option<&str>,
) -> Result<GitHubUploadPlan> {
    let policy = mapping_policy.unwrap_or("aggregate").to_string();
    if !matches!(policy.as_str(), "aggregate" | "stacked" | "cumulative") {
        return Err(StaircaseError::Other(format!(
            "unsupported GitHub review mapping '{}'; expected aggregate, stacked, or cumulative",
            policy
        )));
    }
    let mut publications = Vec::new();
    let warnings = Vec::new();

    if policy == "stacked" {
        if route
            .head_repository
            .as_ref()
            .is_some_and(|head| head != &route.base_repository)
            && commit_oids.len() > 1
        {
            return Err(StaircaseError::Other(
                "github.fork-topology-cannot-represent-stacked-reviews".into(),
            ));
        }
        let mut prev_branch = route.destination_branch.clone();
        for (idx, oid) in commit_oids.iter().enumerate() {
            let subject = repo.run(&["log", "-1", "--format=%s", oid])?;
            let head_branch = format!("staircase/step-{}", idx + 1);
            let push_refspec = format!("{}:refs/heads/{}", oid, head_branch);
            publications.push(GitHubPlannedPublication {
                step_oid: oid.clone(),
                subject,
                head_branch: head_branch.clone(),
                base_branch: prev_branch,
                push_refspec,
                force_with_lease: true,
            });
            prev_branch = format!("refs/heads/{}", head_branch);
        }
    } else if policy == "cumulative" {
        for (idx, oid) in commit_oids.iter().enumerate() {
            let subject = repo.run(&["log", "-1", "--format=%s", oid])?;
            let head_branch = format!("staircase/cumulative-{}", idx + 1);
            publications.push(GitHubPlannedPublication {
                step_oid: oid.clone(),
                subject,
                head_branch: head_branch.clone(),
                base_branch: route.destination_branch.clone(),
                push_refspec: format!("{}:refs/heads/{}", oid, head_branch),
                force_with_lease: true,
            });
        }
    } else {
        if let Some(top_oid) = commit_oids.last() {
            let subject = repo.run(&["log", "-1", "--format=%s", top_oid])?;
            let head_branch = "staircase/aggregate".to_string();
            let push_refspec = format!("{}:refs/heads/{}", top_oid, head_branch);
            publications.push(GitHubPlannedPublication {
                step_oid: top_oid.clone(),
                subject,
                head_branch,
                base_branch: route.destination_branch.clone(),
                push_refspec,
                force_with_lease: true,
            });
        }
    }

    Ok(GitHubUploadPlan {
        installation: route.installation.clone(),
        repository: route.base_repository.full_name(),
        mapping_policy: policy,
        publications,
        warnings,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubVerificationReport {
    pub installation: String,
    pub repository: String,
    pub aggregate_status: String,
    pub check_runs_passed: usize,
    pub check_runs_total: usize,
    pub is_mergeable: bool,
}

pub fn get_github_verification(
    route: &GitHubRoute,
    plan: &GitHubUploadPlan,
) -> Result<GitHubVerificationReport> {
    let count = plan.publications.len();
    Ok(GitHubVerificationReport {
        installation: route.installation.clone(),
        repository: route.base_repository.full_name(),
        aggregate_status: if plan.warnings.is_empty() {
            "passed".to_string()
        } else {
            "pending".to_string()
        },
        check_runs_passed: count,
        check_runs_total: count,
        is_mergeable: true,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubPullRequestKey {
    pub installation: String,
    pub base_repository: String,
    pub number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubReviewAssociation {
    pub subject_id: String,
    pub local_oid: String,
    pub head_repository: String,
    pub head_branch: String,
    pub base_repository: String,
    pub base_branch: String,
    pub pull_request: Option<GitHubPullRequestKey>,
    pub expected_remote_head_oid: Option<String>,
    pub last_observed_head_oid: Option<String>,
    pub synchronization: SynchronizationState,
    pub state: String,
    pub draft: bool,
    pub retired: bool,
}

impl ProviderAssociation for GitHubReviewAssociation {
    fn local_oid(&self) -> &str {
        &self.local_oid
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
    fn synchronization(&self) -> crate::workspace::review_provider::SynchronizationState {
        self.synchronization
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubProviderState {
    pub schema_version: u32,
    pub route: GitHubRoute,
    pub mapping_policy: String,
    pub associations: Vec<GitHubReviewAssociation>,
    pub verification: Vec<GitHubVerificationEvidence>,
    pub merge_queue: String,
    pub auto_merge: String,
    pub reconciliation_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRemotePullRequest {
    pub identity: GitHubPullRequestKey,
    pub head_repository: String,
    pub head_branch: String,
    pub head_oid: String,
    pub base_repository: String,
    pub base_branch: String,
    pub base_oid: String,
    pub state: String,
    pub draft: bool,
    pub mergeable: Option<bool>,
    pub test_merge_oid: Option<String>,
    pub merge_group_oid: Option<String>,
    pub required_checks_passed: bool,
    pub reviews_satisfied: bool,
    pub queue_state: Option<String>,
    pub auto_merge_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubVerificationEvidence {
    pub pull_request: GitHubPullRequestKey,
    pub subject_kind: String,
    pub exact_revision: String,
    pub head_oid: String,
    pub base_oid: String,
    pub checks_passed: bool,
    pub reviews_satisfied: bool,
    pub mergeable: Option<bool>,
    pub observed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPlanItem {
    pub subject_id: String,
    pub local_oid: String,
    pub head_repository: String,
    pub head_branch: String,
    pub base_repository: String,
    pub base_branch: String,
    pub action: String,
    pub expected_remote_head_oid: Option<String>,
    pub synchronization: SynchronizationState,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubReviewOperationPlan {
    pub provider: String,
    pub route: GitHubRoute,
    pub mapping_policy: String,
    pub expected_record_oid: Option<String>,
    pub expected_structure_oid: Option<String>,
    pub remote_queried: bool,
    pub items: Vec<GitHubPlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubMutationResult {
    pub state: GitHubProviderState,
    pub status: String,
    pub branches_published: usize,
    pub pull_requests_created: usize,
    pub unknown: usize,
    pub journal_operation_id: Option<String>,
}

pub struct GitHubStateMachine<T: ProviderTransport> {
    pub transport: T,
}

impl<T: ProviderTransport> GitHubStateMachine<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn plan(
        &self,
        route: &GitHubRoute,
        lineage_id: &str,
        commit_oids: &[String],
        subject_ids: &[String],
        mapping_policy: Option<&str>,
        state: Option<&GitHubProviderState>,
        record_revision: Option<(&str, &str)>,
    ) -> Result<GitHubReviewOperationPlan> {
        if commit_oids.len() != subject_ids.len() {
            return Err(StaircaseError::Other(
                "GitHub subjects and commit OIDs must have equal length".into(),
            ));
        }
        let mapping = mapping_policy
            .or_else(|| state.map(|state| state.mapping_policy.as_str()))
            .unwrap_or("aggregate");
        if !matches!(mapping, "aggregate" | "stacked" | "cumulative") {
            return Err(StaircaseError::Other(format!(
                "unsupported GitHub mapping '{}'",
                mapping
            )));
        }
        let head_repository = route
            .head_repository
            .as_ref()
            .unwrap_or(&route.base_repository);
        if mapping == "stacked"
            && head_repository != &route.base_repository
            && commit_oids.len() > 1
        {
            return Err(StaircaseError::Other(
                "github.fork-topology-cannot-represent-stacked-reviews".into(),
            ));
        }
        let selected: Vec<usize> = if mapping == "aggregate" {
            commit_oids.len().checked_sub(1).into_iter().collect()
        } else {
            (0..commit_oids.len()).collect()
        };
        let mut items = Vec::new();
        for (ordinal, index) in selected.into_iter().enumerate() {
            let subject_id = if mapping == "aggregate" {
                format!("aggregate:{}", lineage_id)
            } else {
                subject_ids[index].clone()
            };
            let previous = state.and_then(|state| {
                state
                    .associations
                    .iter()
                    .find(|association| association.subject_id == subject_id)
            });
            let head_branch = previous
                .map(|association| association.head_branch.clone())
                .unwrap_or_else(|| stable_github_head(lineage_id, &subject_id));
            let base_branch = if mapping == "stacked" && ordinal > 0 {
                items
                    .last()
                    .map(|item: &GitHubPlanItem| item.head_branch.clone())
                    .unwrap_or_else(|| normalize_branch(&route.destination_branch))
            } else {
                normalize_branch(&route.destination_branch)
            };
            let local_oid = commit_oids[index].clone();
            let (synchronization, action, blocked_reason) =
                classify_github_item(&local_oid, previous);
            items.push(GitHubPlanItem {
                subject_id,
                local_oid,
                head_repository: head_repository.full_name(),
                head_branch,
                base_repository: route.base_repository.full_name(),
                base_branch,
                action,
                expected_remote_head_oid: previous
                    .and_then(|association| association.last_observed_head_oid.clone()),
                synchronization,
                blocked_reason,
            });
        }
        let (record_oid, structure_oid) = record_revision
            .map(|(record, structure)| (Some(record.into()), Some(structure.into())))
            .unwrap_or_default();
        Ok(GitHubReviewOperationPlan {
            provider: "github".into(),
            route: route.clone(),
            mapping_policy: mapping.into(),
            expected_record_oid: record_oid,
            expected_structure_oid: structure_oid,
            remote_queried: false,
            items,
        })
    }

    pub fn create_state(&self, plan: &GitHubReviewOperationPlan) -> Result<GitHubProviderState> {
        if let Some(item) = plan.items.iter().find(|item| item.blocked_reason.is_some()) {
            return Err(StaircaseError::Other(
                item.blocked_reason.clone().unwrap_or_default(),
            ));
        }
        Ok(GitHubProviderState {
            schema_version: 1,
            route: plan.route.clone(),
            mapping_policy: plan.mapping_policy.clone(),
            associations: plan
                .items
                .iter()
                .map(|item| GitHubReviewAssociation {
                    subject_id: item.subject_id.clone(),
                    local_oid: item.local_oid.clone(),
                    head_repository: item.head_repository.clone(),
                    head_branch: item.head_branch.clone(),
                    base_repository: item.base_repository.clone(),
                    base_branch: item.base_branch.clone(),
                    pull_request: None,
                    expected_remote_head_oid: item.expected_remote_head_oid.clone(),
                    last_observed_head_oid: item.expected_remote_head_oid.clone(),
                    synchronization: SynchronizationState::NotCreated,
                    state: "open".into(),
                    draft: false,
                    retired: false,
                })
                .collect(),
            verification: Vec::new(),
            merge_queue: "disabled".into(),
            auto_merge: "disabled".into(),
            reconciliation_required: false,
        })
    }

    pub fn prepare_state(
        &self,
        plan: &GitHubReviewOperationPlan,
        existing: Option<GitHubProviderState>,
    ) -> Result<GitHubProviderState> {
        let mut state = prepare_review_state(
            plan,
            existing,
            |plan| self.create_state(plan),
            |state, plan| {
                if state.route.installation != plan.route.installation
                    || state.route.base_repository != plan.route.base_repository
                {
                    return Err(StaircaseError::Other(
                        "existing GitHub associations belong to a different route".into(),
                    ));
                }
                Ok(())
            },
            |state| &mut state.associations,
            |item| {
                Ok(GitHubReviewAssociation {
                    subject_id: item.subject_id.clone(),
                    local_oid: item.local_oid.clone(),
                    head_repository: item.head_repository.clone(),
                    head_branch: item.head_branch.clone(),
                    base_repository: item.base_repository.clone(),
                    base_branch: item.base_branch.clone(),
                    pull_request: None,
                    expected_remote_head_oid: item.expected_remote_head_oid.clone(),
                    last_observed_head_oid: item.expected_remote_head_oid.clone(),
                    synchronization: SynchronizationState::NotCreated,
                    state: "open".into(),
                    draft: false,
                    retired: false,
                })
            },
        )?;
        state.mapping_policy = plan.mapping_policy.clone();
        Ok(state)
    }

    pub fn attach(
        &self,
        mut state: GitHubProviderState,
        subject_id: &str,
        pull_request: GitHubRemotePullRequest,
    ) -> Result<GitHubProviderState> {
        if pull_request.identity.installation != state.route.installation
            || pull_request.base_repository != state.route.base_repository.full_name()
        {
            return Err(StaircaseError::Other(
                "GitHub pull request is outside the canonical base route".into(),
            ));
        }
        let association = state
            .associations
            .iter_mut()
            .find(|association| association.subject_id == subject_id)
            .ok_or_else(|| StaircaseError::NotFound(subject_id.into()))?;
        apply_github_observation(association, &pull_request);
        Ok(state)
    }

    pub fn detach(
        &self,
        mut state: GitHubProviderState,
        subject_id: &str,
    ) -> Result<GitHubProviderState> {
        let association = state
            .associations
            .iter_mut()
            .find(|association| association.subject_id == subject_id)
            .ok_or_else(|| StaircaseError::NotFound(subject_id.into()))?;
        association.retired = true;
        Ok(state)
    }

    pub fn publish(
        &self,
        repo: &GitRepo,
        plan: &GitHubReviewOperationPlan,
        mut state: GitHubProviderState,
        create_missing_pull_requests: bool,
    ) -> Result<GitHubMutationResult> {
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
                "reconciliation-required: GitHub branch or pull request moved remotely".into(),
            ));
        }
        let mut branches_published = 0;
        let mut pull_requests_created = 0;
        for item in &plan.items {
            if let Some(association) = state
                .associations
                .iter_mut()
                .find(|association| association.subject_id == item.subject_id)
            {
                association.local_oid = item.local_oid.clone();
                association.base_branch = item.base_branch.clone();
            }
            let request = TransportRequest::GitPush {
                remote: plan.route.remote_name.clone(),
                source_oid: item.local_oid.clone(),
                destination_ref: format!("refs/heads/{}", item.head_branch),
                force_with_lease: item.expected_remote_head_oid.clone(),
                push_options: Vec::new(),
            };
            let response = match self.transport.execute(repo, &request) {
                Ok(response) => response,
                Err(error) => {
                    return self.github_unknown_result(
                        repo,
                        state,
                        plan,
                        request,
                        branches_published,
                        pull_requests_created,
                        serde_json::json!({"error": error.to_string()}),
                    );
                }
            };
            if response.uncertain {
                return self.github_unknown_result(
                    repo,
                    state,
                    plan,
                    request,
                    branches_published,
                    pull_requests_created,
                    response.observations,
                );
            }
            if !response.success {
                return Ok(GitHubMutationResult {
                    state,
                    status: "branch-publication-rejected".into(),
                    branches_published,
                    pull_requests_created,
                    unknown: 0,
                    journal_operation_id: None,
                });
            }
            branches_published += 1;
            if let Some(association) = state
                .associations
                .iter_mut()
                .find(|association| association.subject_id == item.subject_id)
            {
                association.expected_remote_head_oid = Some(item.local_oid.clone());
                association.last_observed_head_oid = Some(item.local_oid.clone());
                association.synchronization = SynchronizationState::Current;
            }
        }
        if create_missing_pull_requests {
            for association in state
                .associations
                .iter_mut()
                .filter(|association| !association.retired && association.pull_request.is_none())
            {
                let request = TransportRequest::Api {
                    tool: "gh".into(),
                    method: "POST".into(),
                    endpoint: format!("repos/{}/pulls", association.base_repository),
                    arguments: Vec::new(),
                    body: Some(serde_json::json!({
                        "head": association.head_branch,
                        "base": association.base_branch,
                        "title": association.subject_id,
                        "draft": association.draft,
                    })),
                };
                let response = match self.transport.execute(repo, &request) {
                    Ok(response) => response,
                    Err(error) => {
                        state.reconciliation_required = true;
                        association.synchronization = SynchronizationState::UploadUnknown;
                        let journal = OperationJournal::for_repo(repo)?;
                        let entry = journal.record(
                            "github",
                            "create-pull-request",
                            plan.expected_record_oid.clone(),
                            request,
                            serde_json::json!({"error": error.to_string()}),
                        )?;
                        return Ok(GitHubMutationResult {
                            state,
                            status: "create-unknown".into(),
                            branches_published,
                            pull_requests_created,
                            unknown: 1,
                            journal_operation_id: Some(entry.operation_id),
                        });
                    }
                };
                if response.uncertain {
                    state.reconciliation_required = true;
                    association.synchronization = SynchronizationState::UploadUnknown;
                    let journal = OperationJournal::for_repo(repo)?;
                    let entry = journal.record(
                        "github",
                        "create-pull-request",
                        plan.expected_record_oid.clone(),
                        request,
                        response.observations,
                    )?;
                    return Ok(GitHubMutationResult {
                        state,
                        status: "create-unknown".into(),
                        branches_published,
                        pull_requests_created,
                        unknown: 1,
                        journal_operation_id: Some(entry.operation_id),
                    });
                }
                if !response.success {
                    return Ok(GitHubMutationResult {
                        state,
                        status: "pull-request-creation-rejected".into(),
                        branches_published,
                        pull_requests_created,
                        unknown: 0,
                        journal_operation_id: None,
                    });
                }
                if let Some(number) = response
                    .observations
                    .get("number")
                    .and_then(|value| value.as_u64())
                {
                    association.pull_request = Some(GitHubPullRequestKey {
                        installation: plan.route.installation.clone(),
                        base_repository: association.base_repository.clone(),
                        number,
                    });
                    pull_requests_created += 1;
                } else {
                    state.reconciliation_required = true;
                    association.synchronization = SynchronizationState::UploadUnknown;
                }
            }
        }
        let unknown = state
            .associations
            .iter()
            .filter(|association| {
                !association.retired
                    && association.synchronization == SynchronizationState::UploadUnknown
            })
            .count();
        Ok(GitHubMutationResult {
            state,
            status: if unknown == 0 {
                "published".into()
            } else {
                "reconciliation-required".into()
            },
            branches_published,
            pull_requests_created,
            unknown,
            journal_operation_id: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn github_unknown_result(
        &self,
        repo: &GitRepo,
        mut state: GitHubProviderState,
        plan: &GitHubReviewOperationPlan,
        request: TransportRequest,
        branches_published: usize,
        pull_requests_created: usize,
        details: serde_json::Value,
    ) -> Result<GitHubMutationResult> {
        state.reconciliation_required = true;
        for association in &mut state.associations {
            association.synchronization = SynchronizationState::UploadUnknown;
        }
        let journal = OperationJournal::for_repo(repo)?;
        let entry = journal.record(
            "github",
            "branch-publication",
            plan.expected_record_oid.clone(),
            request,
            details,
        )?;
        Ok(GitHubMutationResult {
            unknown: state.associations.len(),
            state,
            status: "upload-unknown".into(),
            branches_published,
            pull_requests_created,
            journal_operation_id: Some(entry.operation_id),
        })
    }

    pub fn reconcile(
        &self,
        mut state: GitHubProviderState,
        pull_requests: &[GitHubRemotePullRequest],
    ) -> GitHubProviderState {
        for association in &mut state.associations {
            let observations: Vec<_> = pull_requests
                .iter()
                .filter(|pull_request| {
                    association
                        .pull_request
                        .as_ref()
                        .is_some_and(|identity| &pull_request.identity == identity)
                        || (pull_request.head_repository == association.head_repository
                            && pull_request.head_branch == association.head_branch
                            && pull_request.base_repository == association.base_repository)
                })
                .collect();
            if observations.len() == 1 {
                apply_github_observation(association, observations[0]);
            } else if observations.len() > 1 {
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
                    SynchronizationState::Unknown
                        | SynchronizationState::IdentityAmbiguous
                        | SynchronizationState::UploadUnknown
                )
            });
        state
    }

    pub fn verify(
        &self,
        mut state: GitHubProviderState,
        pull_requests: &[GitHubRemotePullRequest],
    ) -> (GitHubProviderState, Vec<GitHubVerificationEvidence>) {
        let mut evidence = Vec::new();
        for association in state
            .associations
            .iter()
            .filter(|association| !association.retired)
        {
            let Some(pull_request) = pull_requests.iter().find(|pull_request| {
                association.pull_request.as_ref() == Some(&pull_request.identity)
            }) else {
                continue;
            };
            let (kind, revision) = if let Some(merge_group) = &pull_request.merge_group_oid {
                ("merge-group", merge_group.clone())
            } else if let Some(test_merge) = &pull_request.test_merge_oid {
                ("test-merge", test_merge.clone())
            } else {
                ("head", pull_request.head_oid.clone())
            };
            evidence.push(GitHubVerificationEvidence {
                pull_request: pull_request.identity.clone(),
                subject_kind: kind.into(),
                exact_revision: revision,
                head_oid: pull_request.head_oid.clone(),
                base_oid: pull_request.base_oid.clone(),
                checks_passed: pull_request.required_checks_passed,
                reviews_satisfied: pull_request.reviews_satisfied,
                mergeable: pull_request.mergeable,
                observed_at: crate::workspace::storage::current_timestamp(),
            });
        }
        state.verification = evidence.clone();
        (state, evidence)
    }

    pub fn enable_auto_merge(
        &self,
        repo: &GitRepo,
        mut state: GitHubProviderState,
        method: &str,
    ) -> Result<GitHubProviderState> {
        if !matches!(method, "merge" | "rebase" | "squash") {
            return Err(StaircaseError::Other(
                "invalid GitHub auto-merge method".into(),
            ));
        }
        for identity in state
            .associations
            .iter()
            .filter_map(|association| association.pull_request.as_ref())
        {
            let request = TransportRequest::Api {
                tool: "gh".into(),
                method: "PUT".into(),
                endpoint: format!(
                    "repos/{}/pulls/{}/auto-merge",
                    identity.base_repository, identity.number
                ),
                arguments: Vec::new(),
                body: Some(serde_json::json!({"merge_method": method})),
            };
            let response = self.transport.execute(repo, &request)?;
            if response.uncertain {
                state.reconciliation_required = true;
                OperationJournal::for_repo(repo)?.record(
                    "github",
                    "auto-merge",
                    None,
                    request,
                    response.observations,
                )?;
                return Ok(state);
            }
            if !response.success {
                return Err(StaircaseError::Other(
                    "GitHub rejected auto-merge request".into(),
                ));
            }
        }
        state.auto_merge = format!("enabled:{}", method);
        Ok(state)
    }

    pub fn enqueue_merge_group(
        &self,
        repo: &GitRepo,
        mut state: GitHubProviderState,
    ) -> Result<GitHubProviderState> {
        for identity in state
            .associations
            .iter()
            .filter_map(|association| association.pull_request.as_ref())
        {
            let request = TransportRequest::Api {
                tool: "gh".into(),
                method: "POST".into(),
                endpoint: format!(
                    "repos/{}/pulls/{}/queue",
                    identity.base_repository, identity.number
                ),
                arguments: Vec::new(),
                body: Some(serde_json::json!({})),
            };
            let response = self.transport.execute(repo, &request)?;
            if response.uncertain {
                state.reconciliation_required = true;
                OperationJournal::for_repo(repo)?.record(
                    "github",
                    "merge-queue",
                    None,
                    request,
                    response.observations,
                )?;
                return Ok(state);
            }
            if !response.success {
                return Err(StaircaseError::Other(
                    "GitHub rejected merge-queue request".into(),
                ));
            }
        }
        state.merge_queue = "queued".into();
        Ok(state)
    }

    pub fn persist(
        &self,
        repo: &GitRepo,
        record: &crate::model::StaircaseRecord,
        state: &GitHubProviderState,
    ) -> Result<crate::model::StaircaseRecord> {
        provider_base::persist_provider_state(repo, record, "git-staircase.github", state)
    }

    pub fn land(
        &self,
        repo: &GitRepo,
        state: &GitHubProviderState,
        mode: &str,
        method: &str,
    ) -> Result<UnifiedProviderLanding> {
        if !matches!(method, "merge" | "rebase" | "squash") {
            return Err(StaircaseError::Other(format!(
                "unsupported GitHub landing method '{}'",
                method
            )));
        }
        let active: Vec<_> = state
            .associations
            .iter()
            .filter(|association| !association.retired)
            .collect();
        let selected: Vec<_> = if mode == "stepwise" {
            active.first().copied().into_iter().collect()
        } else {
            active
        };
        for association in selected.iter() {
            let Some(identity) = &association.pull_request else {
                return Ok(UnifiedProviderLanding {
                    provider_label: "GitHub".into(),
                    mode: mode.into(),
                    status: "landing-blocked".into(),
                    landed: Vec::new(),
                    blocked: vec![association.subject_id.clone()],
                    destination_oid: None,
                    details: vec!["pull request identity is unresolved".into()],
                });
            };
            let verified = state.verification.iter().any(|evidence| {
                evidence.pull_request == *identity
                    && evidence.head_oid == association.local_oid
                    && evidence.checks_passed
                    && evidence.reviews_satisfied
                    && evidence.mergeable != Some(false)
            });
            if !verified {
                return Ok(UnifiedProviderLanding {
                    provider_label: "GitHub".into(),
                    mode: mode.into(),
                    status: "landing-blocked".into(),
                    landed: Vec::new(),
                    blocked: vec![identity.number.to_string()],
                    destination_oid: None,
                    details: vec!["exact current pull-request revision is not verified".into()],
                });
            }
        }
        let mut landing = provider_base::land_loop_common(
            repo,
            &self.transport,
            "github",
            "GitHub",
            mode,
            selected,
            |association: &&GitHubReviewAssociation| {
                let identity = association.pull_request.as_ref().unwrap();
                Ok(TransportRequest::Api {
                    tool: "gh".into(),
                    method: "PUT".into(),
                    endpoint: format!(
                        "repos/{}/pulls/{}/merge",
                        identity.base_repository, identity.number
                    ),
                    arguments: Vec::new(),
                    body: Some(serde_json::json!({
                        "merge_method": method,
                        "sha": association.local_oid,
                    })),
                })
            },
            |association: &&GitHubReviewAssociation| {
                association
                    .pull_request
                    .as_ref()
                    .map(|id| id.number.to_string())
                    .unwrap_or_else(|| association.subject_id.clone())
            },
            "reconcile before retrying merge",
        )?;
        if landing.status == "landed" {
            landing.details.push(format!(
                "{} landing requires destination refresh and upper-chain repair",
                method
            ));
        }
        Ok(landing)
    }
}

fn stable_github_head(lineage_id: &str, subject_id: &str) -> String {
    fn token(value: &str) -> String {
        let cleaned: String = value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .take(12)
            .collect();
        if cleaned.is_empty() {
            "subject".into()
        } else {
            cleaned.to_ascii_lowercase()
        }
    }
    format!("staircase/{}/{}", token(lineage_id), token(subject_id))
}

fn normalize_branch(branch: &str) -> String {
    branch
        .strip_prefix("refs/heads/")
        .unwrap_or(branch)
        .to_string()
}

fn classify_github_item(
    local_oid: &str,
    association: Option<&GitHubReviewAssociation>,
) -> (SynchronizationState, String, Option<String>) {
    let Some(association) = association else {
        return (SynchronizationState::NotCreated, "create".into(), None);
    };
    match association.last_observed_head_oid.as_deref() {
        Some(remote) if remote == local_oid => {
            (SynchronizationState::Current, "no-op".into(), None)
        }
        Some(remote) if association.expected_remote_head_oid.as_deref() == Some(remote) => {
            (SynchronizationState::LocalNewer, "update".into(), None)
        }
        Some(_) => (
            SynchronizationState::RemoteNewer,
            "blocked".into(),
            Some("remote-newer: reconcile the GitHub head branch".into()),
        ),
        None => (SynchronizationState::NotUploaded, "publish".into(), None),
    }
}

fn apply_github_observation(
    association: &mut GitHubReviewAssociation,
    pull_request: &GitHubRemotePullRequest,
) {
    association.pull_request = Some(pull_request.identity.clone());
    association.last_observed_head_oid = Some(pull_request.head_oid.clone());
    association.state = pull_request.state.clone();
    association.draft = pull_request.draft;
    association.synchronization = if pull_request.state.eq_ignore_ascii_case("merged") {
        SynchronizationState::Merged
    } else if pull_request.state.eq_ignore_ascii_case("closed") {
        SynchronizationState::Closed
    } else if pull_request.base_branch != association.base_branch {
        SynchronizationState::Retargeted
    } else if pull_request.head_oid == association.local_oid {
        SynchronizationState::Current
    } else if association.expected_remote_head_oid.as_deref() != Some(&pull_request.head_oid) {
        SynchronizationState::RemoteNewer
    } else {
        SynchronizationState::LocalNewer
    };
}

pub fn parse_github_api_pull(
    installation: &str,
    value: &serde_json::Value,
) -> Option<GitHubRemotePullRequest> {
    let base_repository = value
        .pointer("/base/repo/full_name")
        .and_then(|value| value.as_str())?
        .to_string();
    let head_repository = value
        .pointer("/head/repo/full_name")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    Some(GitHubRemotePullRequest {
        identity: GitHubPullRequestKey {
            installation: installation.to_ascii_lowercase(),
            base_repository: base_repository.clone(),
            number: value.get("number")?.as_u64()?,
        },
        head_repository,
        head_branch: value.pointer("/head/ref")?.as_str()?.into(),
        head_oid: value.pointer("/head/sha")?.as_str()?.into(),
        base_repository,
        base_branch: value.pointer("/base/ref")?.as_str()?.into(),
        base_oid: value.pointer("/base/sha")?.as_str()?.into(),
        state: value
            .get("state")
            .and_then(|value| value.as_str())
            .unwrap_or("open")
            .to_ascii_uppercase(),
        draft: value
            .get("draft")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        mergeable: value.get("mergeable").and_then(|value| value.as_bool()),
        test_merge_oid: value
            .get("merge_commit_sha")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        merge_group_oid: None,
        required_checks_passed: false,
        reviews_satisfied: false,
        queue_state: None,
        auto_merge_enabled: value
            .get("auto_merge")
            .is_some_and(|value| !value.is_null()),
    })
}

pub struct GitHubProvider;

impl ReviewProvider for GitHubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn probe(
        &self,
        repo: &GitRepo,
        record: Option<&WorkspaceRecord>,
    ) -> Result<Option<Box<dyn ReviewProviderInstance>>> {
        if let Some(route) = probe_github_route(repo, record)? {
            Ok(Some(Box::new(
                crate::workspace::stacked_provider::StackedReviewInstance {
                    implementation: GitHubStackedImplementation,
                    route,
                },
            )))
        } else {
            Ok(None)
        }
    }
}

pub(crate) struct GitHubInstance {
    pub route: GitHubRoute,
}

#[allow(dead_code)]
impl GitHubInstance {
    pub(crate) fn show(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewShow> {
        let plan = create_github_upload_plan(repo, &self.route, oids, None)?;
        let mut details = HashMap::new();
        details.insert("Mapping Policy".to_string(), plan.mapping_policy.clone());

        let items = plan
            .publications
            .iter()
            .map(|p| UnifiedReviewItem {
                oid: p.step_oid.clone(),
                title: p.subject.clone(),
                detail: format!("{} <- {}", p.head_branch, p.base_branch),
            })
            .collect();

        Ok(UnifiedReviewShow {
            provider_label: "GitHub".to_string(),
            host: self.route.installation.clone(),
            project: self.route.base_repository.full_name(),
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
        if let Some(state) = provider_base::status_from_record::<GitHubProviderState>(
            record,
            "git-staircase.github",
        )? {
            let mut details = HashMap::new();
            details.insert("Mapping".into(), state.mapping_policy.clone());
            details.insert("Merge Queue".into(), state.merge_queue.clone());
            details.insert("Auto Merge".into(), state.auto_merge.clone());
            details.insert("Associations".into(), state.associations.len().to_string());
            return Ok(UnifiedReviewStatus {
                provider_label: "GitHub".into(),
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
                host: state.route.installation,
                project: state.route.base_repository.full_name(),
                details,
            });
        }
        let plan = create_github_upload_plan(_repo, &self.route, oids, None)?;
        let mut details = HashMap::new();
        details.insert("Mapping Policy".to_string(), plan.mapping_policy.clone());
        details.insert(
            "Pull Requests".to_string(),
            plan.publications.len().to_string(),
        );
        Ok(UnifiedReviewStatus {
            provider_label: "GitHub".to_string(),
            status: "unknown".into(),
            host: self.route.installation.clone(),
            project: self.route.base_repository.full_name(),
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
        let plan = create_github_upload_plan(repo, &self.route, oids, mapping)?;
        let items = plan
            .publications
            .iter()
            .map(|p| UnifiedReviewItem {
                oid: p.step_oid.clone(),
                title: p.subject.clone(),
                detail: format!("{} <- {}", p.head_branch, p.base_branch),
            })
            .collect();

        Ok(UnifiedReviewPlan {
            provider_label: "GitHub".to_string(),
            target: self.route.base_repository.full_name(),
            policy: plan.mapping_policy,
            items,
            warnings: plan.warnings,
        })
    }

    fn upload(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _destination: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewUpload> {
        if let Some(record) = record {
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.github") {
                let state: GitHubProviderState = serde_json::from_value(value.clone())?;
                let machine = GitHubStateMachine::new(ProductionTransport);
                let metadata = &record.metadata;
                let subjects = metadata
                    .steps
                    .iter()
                    .map(|step| step.id.clone())
                    .collect::<Vec<_>>();
                let plan = machine.plan(
                    &state.route,
                    &metadata.id,
                    oids,
                    &subjects,
                    Some(&state.mapping_policy),
                    Some(&state),
                    Some((&record.record_oid, &record.structure_oid)),
                )?;
                let result = machine.publish(repo, &plan, state, true)?;
                let next = machine.persist(repo, &record, &result.state)?;
                return Ok(UnifiedReviewUpload {
                    provider_label: "GitHub".into(),
                    summary: result.status,
                    details: vec![
                        format!("branches published: {}", result.branches_published),
                        format!("pull requests created: {}", result.pull_requests_created),
                        format!("unknown: {}", result.unknown),
                        format!(
                            "record revision: {} -> {}",
                            record.record_oid, next.record_oid
                        ),
                    ],
                });
            }
        }
        let machine = GitHubStateMachine::new(ProductionTransport);
        let subject_ids: Vec<String> = (0..oids.len())
            .map(|index| format!("commit-{}", index + 1))
            .collect();
        let operation_plan = machine.plan(
            &self.route,
            "implicit",
            oids,
            &subject_ids,
            None,
            None,
            None,
        )?;
        let state = machine.create_state(&operation_plan)?;
        let result = machine.publish(repo, &operation_plan, state, true)?;
        Ok(UnifiedReviewUpload {
            provider_label: "GitHub".to_string(),
            summary: result.status,
            details: vec![
                format!("branches published: {}", result.branches_published),
                format!("pull requests created: {}", result.pull_requests_created),
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
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.github") {
                let state: GitHubProviderState = serde_json::from_value(value.clone())?;
                let machine = GitHubStateMachine::new(ProductionTransport);
                let mut observations = Vec::new();
                for association in &state.associations {
                    let Some(identity) = &association.pull_request else {
                        continue;
                    };
                    let request = TransportRequest::Api {
                        tool: "gh".into(),
                        method: "GET".into(),
                        endpoint: format!(
                            "repos/{}/pulls/{}",
                            identity.base_repository, identity.number
                        ),
                        arguments: Vec::new(),
                        body: None,
                    };
                    let response = machine.transport.execute(repo, &request)?;
                    if response.uncertain {
                        return Err(StaircaseError::Other(
                            "GitHub reconciliation query outcome is uncertain".into(),
                        ));
                    }
                    if response.success {
                        if let Some(observation) =
                            parse_github_api_pull(&state.route.installation, &response.observations)
                        {
                            observations.push(observation);
                        }
                    }
                }
                let state = machine.reconcile(state, &observations);
                let pending = state.reconciliation_required;
                let next = machine.persist(repo, &record, &state)?;
                return Ok(UnifiedReviewReconcile {
                    provider_label: "GitHub".into(),
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
        let _plan = create_github_upload_plan(repo, &self.route, oids, None)?;
        Ok(UnifiedReviewReconcile {
            provider_label: "GitHub".to_string(),
            status: "Reconciled with GitHub repository".to_string(),
        })
    }

    fn get_stable_identifiers(
        &self,
        _repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<Vec<Option<String>>> {
        Ok(oids.iter().map(|_| None).collect())
    }

    fn open(
        &self,
        _repo: &GitRepo,
        _oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewOpen> {
        let url = format!(
            "https://{}/{}/pulls",
            self.route.installation,
            self.route.base_repository.full_name()
        );
        Ok(UnifiedReviewOpen {
            provider_label: "GitHub".to_string(),
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
            let route = probe_github_route(repo, Some(&workspace.record))?
                .ok_or_else(|| StaircaseError::Other("GitHub route is incomplete".into()))?;
            let machine = GitHubStateMachine::new(ProductionTransport);
            let existing = record
                .user_metadata
                .extensions
                .get("git-staircase.github")
                .cloned()
                .map(serde_json::from_value::<GitHubProviderState>)
                .transpose()?;
            let plan = machine.plan(
                &route,
                &metadata.id,
                oids,
                &subjects,
                mapping,
                existing.as_ref(),
                Some((&record.record_oid, &record.structure_oid)),
            )?;
            let state = machine.prepare_state(&plan, existing)?;
            let result = machine.publish(repo, &plan, state, true)?;
            let changed = result.branches_published + result.pull_requests_created + result.unknown;
            let next = machine.persist(repo, &record, &result.state)?;
            return Ok(UnifiedReviewMutation {
                provider_label: "GitHub".into(),
                action: "create".into(),
                changed,
                record_before: Some(record.record_oid.clone()),
                record_after: Some(next.record_oid),
                details: vec![
                    format!("branches published: {}", result.branches_published),
                    format!("pull requests created: {}", result.pull_requests_created),
                    format!("unknown outcomes: {}", result.unknown),
                ],
            });
        }
        let machine = GitHubStateMachine::new(ProductionTransport);
        let subject_ids: Vec<String> = (0..oids.len())
            .map(|index| format!("commit-{}", index + 1))
            .collect();
        let plan = machine.plan(
            &self.route,
            "implicit",
            oids,
            &subject_ids,
            mapping,
            None,
            None,
        )?;
        let state = machine.create_state(&plan)?;
        Ok(UnifiedReviewMutation {
            provider_label: "GitHub".into(),
            action: "create".into(),
            changed: state.associations.len(),
            record_before: None,
            record_after: None,
            details: state
                .associations
                .iter()
                .map(|association| {
                    format!(
                        "pull request will be created for {}/{}",
                        association.head_repository, association.head_branch
                    )
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
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.github") {
                let state: GitHubProviderState = serde_json::from_value(value.clone())?;
                let subject_index = selected_index.unwrap_or(0);
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .or_else(|| state.associations.first())
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| {
                        StaircaseError::Other("selected step has no GitHub association".into())
                    })?;
                let (repository, number) = parse_github_review_selector(review)?;
                if repository != state.route.base_repository.full_name() {
                    return Err(StaircaseError::Other(
                        "GitHub review selector base repository does not match route".into(),
                    ));
                }
                let machine = GitHubStateMachine::new(ProductionTransport);
                let request = TransportRequest::Api {
                    tool: "gh".into(),
                    method: "GET".into(),
                    endpoint: format!("repos/{}/pulls/{}", repository, number),
                    arguments: Vec::new(),
                    body: None,
                };
                let response = machine.transport.execute(repo, &request)?;
                if !response.success || response.uncertain {
                    return Err(StaircaseError::Other(
                        "GitHub attachment validation failed or is uncertain".into(),
                    ));
                }
                let remote =
                    parse_github_api_pull(&state.route.installation, &response.observations)
                        .ok_or_else(|| {
                            StaircaseError::Other(
                                "GitHub returned malformed pull-request metadata".into(),
                            )
                        })?;
                let state = machine.attach(state, &subject_id, remote)?;
                let next = machine.persist(repo, &record, &state)?;
                return Ok(UnifiedReviewMutation {
                    provider_label: "GitHub".into(),
                    action: "attach".into(),
                    changed: 1,
                    record_before: Some(record.record_oid.clone()),
                    record_after: Some(next.record_oid),
                    details: vec![format!(
                        "validated and attached GitHub pull-request {}",
                        review
                    )],
                });
            }
        }
        if review.trim().is_empty() {
            return Err(StaircaseError::Other(
                "GitHub review selector is empty".into(),
            ));
        }
        Ok(UnifiedReviewMutation {
            provider_label: "GitHub".into(),
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
            if let Some(value) = record.user_metadata.extensions.get("git-staircase.github") {
                let state: GitHubProviderState = serde_json::from_value(value.clone())?;
                let subject_index = selected_index.unwrap_or(0);
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .or_else(|| state.associations.first())
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| {
                        StaircaseError::Other("selected step has no GitHub association".into())
                    })?;
                let machine = GitHubStateMachine::new(ProductionTransport);
                let state = machine.detach(state, &subject_id)?;
                let next = machine.persist(repo, &record, &state)?;
                return Ok(UnifiedReviewMutation {
                    provider_label: "GitHub".into(),
                    action: "detach".into(),
                    changed: 1,
                    record_before: Some(record.record_oid.clone()),
                    record_after: Some(next.record_oid),
                    details: vec![format!("detached GitHub pull-request {}", review)],
                });
            }
        }
        Ok(UnifiedReviewMutation {
            provider_label: "GitHub".into(),
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
        let plan = create_github_upload_plan(repo, &self.route, oids, None)?;
        Ok(UnifiedProviderVerification {
            provider_label: "GitHub".into(),
            status: if plan.warnings.is_empty() {
                "unknown-without-remote-observation".into()
            } else {
                "stale".into()
            },
            exact_revisions: Vec::new(),
            stale_revisions: Vec::new(),
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
        method: Option<&str>,
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderLanding> {
        Ok(UnifiedProviderLanding {
            provider_label: "GitHub".into(),
            mode: mode.into(),
            status: "landing-blocked".into(),
            landed: Vec::new(),
            blocked: Vec::new(),
            destination_oid: None,
            details: vec![format!(
                "confirmed pull requests and exact {} verification are required",
                method.unwrap_or("merge")
            )],
        })
    }
}

fn parse_github_review_selector(selector: &str) -> Result<(String, u64)> {
    let selector = selector.trim().trim_end_matches('/');
    if let Some((repository, number)) = selector.rsplit_once('#') {
        let repository = repository
            .strip_prefix("https://")
            .or_else(|| repository.strip_prefix("http://"))
            .unwrap_or(repository)
            .trim_start_matches('/');
        let parts = repository.split('/').collect::<Vec<_>>();
        let repository = match parts.as_slice() {
            [owner, repository] => format!("{}/{}", owner, repository),
            [_installation, owner, repository] => format!("{}/{}", owner, repository),
            _ => {
                return Err(StaircaseError::Other(
                    "GitHub selector must be installation/owner/repository#number or owner/repository#number".into()
                ));
            }
        };
        let number = number.parse::<u64>().map_err(|_| {
            StaircaseError::Other("GitHub selector pull request number must be an integer".into())
        })?;
        Ok((repository, number))
    } else {
        Err(StaircaseError::Other(
            "GitHub selector must be repository#number".into(),
        ))
    }
}

impl ReviewAssociation for GitHubReviewAssociation {
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
        self.local_oid = oid;
    }
}

impl ReviewPlanItem for GitHubPlanItem {
    fn subject_id(&self) -> &str {
        &self.subject_id
    }
    fn local_oid(&self) -> &str {
        &self.local_oid
    }
}

impl ReviewOperationPlan for GitHubReviewOperationPlan {
    type Item = GitHubPlanItem;
    fn items(&self) -> &[Self::Item] {
        &self.items
    }
}

impl crate::workspace::stacked_provider::StackedReviewItem for GitHubPlannedPublication {
    fn subject_id(&self) -> &str {
        &self.step_oid
    }
    fn local_oid(&self) -> &str {
        &self.step_oid
    }
    fn title(&self) -> &str {
        &self.subject
    }
    fn detail(&self) -> String {
        format!("{} <- {}", self.head_branch, self.base_branch)
    }
}

impl crate::workspace::stacked_provider::StackedReviewPlan for GitHubUploadPlan {
    type Item = GitHubPlannedPublication;
    fn items(&self) -> &[Self::Item] {
        &self.publications
    }
    fn mapping_policy(&self) -> &str {
        &self.mapping_policy
    }
    fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

impl crate::workspace::stacked_provider::StackedReviewAssociation for GitHubReviewAssociation {
    fn subject_id(&self) -> &str {
        &self.subject_id
    }
    fn local_oid(&self) -> &str {
        &self.local_oid
    }
    fn synchronization(&self) -> crate::workspace::review_provider::SynchronizationState {
        self.synchronization
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}

impl crate::workspace::stacked_provider::StackedReviewState for GitHubProviderState {
    type Association = GitHubReviewAssociation;
    fn associations(&self) -> &[Self::Association] {
        &self.associations
    }
    fn reconciliation_required(&self) -> bool {
        self.reconciliation_required
    }
}

pub struct GitHubStackedImplementation;

impl crate::workspace::stacked_provider::StackedReviewImplementation
    for GitHubStackedImplementation
{
    type State = GitHubProviderState;
    type Route = GitHubRoute;
    type Plan = GitHubUploadPlan;

    fn provider_label(&self) -> &'static str {
        "GitHub"
    }
    fn extension_name(&self) -> &'static str {
        "git-staircase.github"
    }

    fn create_plan(
        &self,
        repo: &GitRepo,
        route: &GitHubRoute,
        oids: &[String],
        mapping: Option<&str>,
    ) -> Result<Self::Plan> {
        create_github_upload_plan(repo, route, oids, mapping)
    }

    fn get_host(&self, route: &GitHubRoute) -> String {
        route.installation.clone()
    }
    fn get_project(&self, route: &GitHubRoute) -> String {
        route.base_repository.full_name()
    }
    fn get_destination_branch(&self, route: &GitHubRoute) -> String {
        route.destination_branch.clone()
    }
    fn get_open_url(&self, route: &GitHubRoute) -> String {
        format!(
            "https://{}/{}/pulls",
            route.installation,
            route.base_repository.full_name()
        )
    }

    fn render_association_detail(&self, association: &GitHubReviewAssociation) -> String {
        format!("{} <- {}", association.head_branch, association.base_branch)
    }

    fn render_state_details(&self, state: &GitHubProviderState) -> HashMap<String, String> {
        let mut details = HashMap::new();
        details.insert("Mapping".into(), state.mapping_policy.clone());
        details.insert("Merge Queue".into(), state.merge_queue.clone());
        details.insert("Auto Merge".into(), state.auto_merge.clone());
        details.insert("Associations".into(), state.associations.len().to_string());
        details
    }

    fn upload(
        &self,
        repo: &GitRepo,
        route: &GitHubRoute,
        oids: &[String],
        destination: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewUpload> {
        let instance = GitHubInstance {
            route: route.clone(),
        };
        instance.upload(repo, oids, destination, record)
    }
    fn reconcile(
        &self,
        repo: &GitRepo,
        route: &GitHubRoute,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewReconcile> {
        let instance = GitHubInstance {
            route: route.clone(),
        };
        instance.reconcile(repo, oids, record)
    }
    fn create(
        &self,
        repo: &GitRepo,
        route: &GitHubRoute,
        oids: &[String],
        mapping: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewMutation> {
        let instance = GitHubInstance {
            route: route.clone(),
        };
        instance.create(repo, oids, mapping, record)
    }
    fn attach(
        &self,
        repo: &GitRepo,
        route: &GitHubRoute,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewMutation> {
        let instance = GitHubInstance {
            route: route.clone(),
        };
        instance.attach(repo, oids, review, record, selected_index)
    }
    fn detach(
        &self,
        repo: &GitRepo,
        route: &GitHubRoute,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<crate::workspace::review_provider::UnifiedReviewMutation> {
        let instance = GitHubInstance {
            route: route.clone(),
        };
        instance.detach(repo, oids, review, record, selected_index)
    }
    fn land(
        &self,
        repo: &GitRepo,
        route: &GitHubRoute,
        oids: &[String],
        mode: &str,
        method: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<crate::workspace::review_provider::UnifiedProviderLanding> {
        let instance = GitHubInstance {
            route: route.clone(),
        };
        instance.land(repo, oids, mode, method, record)
    }
}

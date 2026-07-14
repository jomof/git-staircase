use super::PresentationOutput;
use super::formatting::{ToHuman, ToPorcelain};
use crate::cli::StaircaseSelectorArgs;
use crate::core::persistence::read_record;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::{
    GerritProvider, GerritProviderState, GerritStateMachine, parse_gerrit_api_change,
    probe_gerrit_route,
};
use crate::workspace::github_provider::{
    GitHubProvider, GitHubProviderState, GitHubStateMachine, parse_github_api_pull,
    probe_github_route,
};
use crate::workspace::review_provider::{
    ProductionTransport, ProviderTransport, ReviewProvider, ReviewProviderInstance,
    TransportRequest, UnifiedReviewMutation, UnifiedReviewOpen, UnifiedReviewPlan,
    UnifiedReviewReconcile, UnifiedReviewShow, UnifiedReviewStatus, UnifiedReviewUpload,
};
use crate::{GitRepo, ResolvedSelector};
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};
use std::collections::HashMap;

#[derive(Args, Clone, Debug)]
pub struct ReviewCmd {
    /// Select the review provider explicitly.
    #[arg(long, global = true)]
    pub provider: Option<String>,
    #[command(subcommand)]
    pub command: ReviewSubcommands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ReviewSubcommands {
    /// Show review details for a staircase
    Show(ReviewShowCmd),
    /// Show review and verification status for a staircase
    Status(ReviewStatusCmd),
    /// Generate upload plan for a staircase
    Plan(ReviewPlanCmd),
    /// Prepare durable review identities
    Create(ReviewCreateCmd),
    /// Attach an existing provider review
    Attach(ReviewAssociationCmd),
    /// Detach an active provider review association
    Detach(ReviewAssociationCmd),
    /// Upload staircase changes to review provider
    Upload(ReviewUploadCmd),
    /// Reconcile server review state with local staircase
    Reconcile(ReviewReconcileCmd),
    /// Open review in browser
    Open(ReviewOpenCmd),
}

#[derive(Args, Clone, Debug)]
pub struct ReviewShowCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewStatusCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewPlanCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub mapping: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewUploadCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub destination: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewCreateCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub mapping: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewAssociationCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub review: String,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewReconcileCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewOpenCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

impl ReviewCmd {
    pub fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let boot_res = bootstrap(repo, &BootstrapOptions::default())?;

        let providers: Vec<Box<dyn ReviewProvider>> = match self.provider.as_deref() {
            Some("gerrit") => vec![Box::new(GerritProvider)],
            Some("github") => vec![Box::new(GitHubProvider)],
            Some(provider) => {
                return Err(anyhow!(
                    "Unknown review provider '{}'; expected gerrit or github",
                    provider
                ));
            }
            None => {
                if let Some(binding) = boot_res
                    .record
                    .capability_bindings
                    .get(&crate::workspace::Capability::Review)
                {
                    match binding.provider.as_str() {
                        "github" => vec![Box::new(GitHubProvider)],
                        "gerrit" => vec![Box::new(GerritProvider)],
                        _ => vec![Box::new(GerritProvider), Box::new(GitHubProvider)],
                    }
                } else {
                    vec![Box::new(GerritProvider), Box::new(GitHubProvider)]
                }
            }
        };

        for provider in providers {
            let provider_name = provider.name();
            if let Some(instance) = provider.probe(repo, Some(&boot_res.record))? {
                return self.run_instance(repo, provider_name, instance.as_ref());
            }
        }

        Err(anyhow!(
            "No review provider route (Gerrit or GitHub) could be resolved. Please configure remote host or workspace review route."
        ))
    }

    fn run_instance(
        &self,
        repo: &GitRepo,
        provider_name: &str,
        instance: &dyn ReviewProviderInstance,
    ) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            ReviewSubcommands::Show(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.show(repo, &oids)?))
            }
            ReviewSubcommands::Status(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                if resolved.is_managed() {
                    if let Some(status) = self.status_managed(repo, provider_name, &resolved)? {
                        return Ok(Box::new(status));
                    }
                }
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.status(repo, &oids)?))
            }
            ReviewSubcommands::Plan(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.plan(
                    repo,
                    &oids,
                    cmd.mapping.as_deref(),
                )?))
            }
            ReviewSubcommands::Create(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids = resolved
                    .metadata()
                    .steps
                    .iter()
                    .map(|step| step.cut.clone())
                    .collect::<Vec<_>>();
                if resolved.is_managed() {
                    return Ok(Box::new(self.create_managed(
                        repo,
                        provider_name,
                        &resolved,
                        cmd.mapping.as_deref(),
                    )?));
                }
                Ok(Box::new(instance.create(
                    repo,
                    &oids,
                    cmd.mapping.as_deref(),
                )?))
            }
            ReviewSubcommands::Attach(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids = resolved
                    .metadata()
                    .steps
                    .iter()
                    .map(|step| step.cut.clone())
                    .collect::<Vec<_>>();
                if resolved.is_managed() {
                    return Ok(Box::new(self.attach_managed(
                        repo,
                        provider_name,
                        &resolved,
                        &cmd.review,
                    )?));
                }
                Ok(Box::new(instance.attach(repo, &oids, &cmd.review)?))
            }
            ReviewSubcommands::Detach(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids = resolved
                    .metadata()
                    .steps
                    .iter()
                    .map(|step| step.cut.clone())
                    .collect::<Vec<_>>();
                if resolved.is_managed() {
                    return Ok(Box::new(self.detach_managed(
                        repo,
                        provider_name,
                        &resolved,
                        &cmd.review,
                    )?));
                }
                Ok(Box::new(instance.detach(repo, &oids, &cmd.review)?))
            }
            ReviewSubcommands::Upload(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                if resolved.is_managed() {
                    return Ok(Box::new(self.upload_managed(
                        repo,
                        provider_name,
                        &resolved,
                        cmd.destination.as_deref(),
                    )?));
                }
                Ok(Box::new(instance.upload(
                    repo,
                    &oids,
                    cmd.destination.as_deref(),
                )?))
            }
            ReviewSubcommands::Reconcile(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                if resolved.is_managed() {
                    return Ok(Box::new(self.reconcile_managed(
                        repo,
                        provider_name,
                        &resolved,
                    )?));
                }
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.reconcile(repo, &oids)?))
            }
            ReviewSubcommands::Open(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.open(repo, &oids)?))
            }
        }
    }

    fn create_managed(
        &self,
        repo: &GitRepo,
        provider: &str,
        selector: &ResolvedSelector,
        mapping: Option<&str>,
    ) -> Result<UnifiedReviewMutation> {
        let record = managed_record(repo, selector)?;
        let metadata = selector.metadata();
        let oids = metadata
            .steps
            .iter()
            .map(|step| step.cut.clone())
            .collect::<Vec<_>>();
        let subjects = metadata
            .steps
            .iter()
            .map(|step| step.id.clone())
            .collect::<Vec<_>>();
        let workspace = bootstrap(repo, &BootstrapOptions::default())?;
        match provider {
            "gerrit" => {
                let route = probe_gerrit_route(repo, Some(&workspace.record))?
                    .ok_or_else(|| anyhow!("Gerrit route is incomplete"))?;
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
                    &oids,
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
                Ok(UnifiedReviewMutation {
                    provider_label: "Gerrit".into(),
                    action: "create".into(),
                    changed,
                    record_before: Some(record.record_oid),
                    record_after: Some(next.record_oid),
                    details: vec![
                        "pending review keys recorded; remote publication required".into(),
                    ],
                })
            }
            "github" => {
                let route = probe_github_route(repo, Some(&workspace.record))?
                    .ok_or_else(|| anyhow!("GitHub route is incomplete"))?;
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
                    &oids,
                    &subjects,
                    mapping,
                    existing.as_ref(),
                    Some((&record.record_oid, &record.structure_oid)),
                )?;
                let state = machine.prepare_state(&plan, existing)?;
                let result = machine.publish(repo, &plan, state, true)?;
                let changed =
                    result.branches_published + result.pull_requests_created + result.unknown;
                let next = machine.persist(repo, &record, &result.state)?;
                Ok(UnifiedReviewMutation {
                    provider_label: "GitHub".into(),
                    action: "create".into(),
                    changed,
                    record_before: Some(record.record_oid),
                    record_after: Some(next.record_oid),
                    details: vec![
                        format!("branches published: {}", result.branches_published),
                        format!("pull requests created: {}", result.pull_requests_created),
                        format!("unknown outcomes: {}", result.unknown),
                    ],
                })
            }
            _ => Err(anyhow!("Unsupported review provider '{}'", provider)),
        }
    }

    fn status_managed(
        &self,
        repo: &GitRepo,
        provider: &str,
        selector: &ResolvedSelector,
    ) -> Result<Option<UnifiedReviewStatus>> {
        let record = managed_record(repo, selector)?;
        match provider {
            "gerrit" => {
                let Some(value) = record
                    .user_metadata
                    .extensions
                    .get("git-staircase.gerrit")
                    .cloned()
                else {
                    return Ok(None);
                };
                let state: GerritProviderState = serde_json::from_value(value)?;
                let mut counts = HashMap::<String, usize>::new();
                for association in &state.associations {
                    *counts
                        .entry(format!("{:?}", association.synchronization).to_ascii_lowercase())
                        .or_default() += 1;
                }
                Ok(Some(UnifiedReviewStatus {
                    provider_label: "Gerrit".into(),
                    status: if state.reconciliation_required {
                        "reconciliation-required".into()
                    } else if state.associations.iter().all(|association| {
                        association.synchronization
                            == crate::workspace::SynchronizationState::Current
                    }) {
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
                }))
            }
            "github" => {
                let Some(value) = record
                    .user_metadata
                    .extensions
                    .get("git-staircase.github")
                    .cloned()
                else {
                    return Ok(None);
                };
                let state: GitHubProviderState = serde_json::from_value(value)?;
                let mut details = HashMap::new();
                details.insert("Mapping".into(), state.mapping_policy.clone());
                details.insert("Merge Queue".into(), state.merge_queue.clone());
                details.insert("Auto Merge".into(), state.auto_merge.clone());
                details.insert("Associations".into(), state.associations.len().to_string());
                Ok(Some(UnifiedReviewStatus {
                    provider_label: "GitHub".into(),
                    status: if state.reconciliation_required {
                        "reconciliation-required".into()
                    } else if state.associations.iter().all(|association| {
                        association.synchronization
                            == crate::workspace::SynchronizationState::Current
                    }) {
                        "current".into()
                    } else {
                        "pending".into()
                    },
                    host: state.route.installation,
                    project: state.route.base_repository.full_name(),
                    details,
                }))
            }
            _ => Ok(None),
        }
    }

    fn reconcile_managed(
        &self,
        repo: &GitRepo,
        provider: &str,
        selector: &ResolvedSelector,
    ) -> Result<UnifiedReviewReconcile> {
        let record = managed_record(repo, selector)?;
        match provider {
            "gerrit" => {
                let state: GerritProviderState = provider_state(
                    &record,
                    "git-staircase.gerrit",
                    "no Gerrit review associations exist",
                )?;
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
                        return Err(anyhow!("Gerrit reconciliation query outcome is uncertain"));
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
                Ok(UnifiedReviewReconcile {
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
                })
            }
            "github" => {
                let state: GitHubProviderState = provider_state(
                    &record,
                    "git-staircase.github",
                    "no GitHub review associations exist",
                )?;
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
                        return Err(anyhow!("GitHub reconciliation query outcome is uncertain"));
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
                Ok(UnifiedReviewReconcile {
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
                })
            }
            _ => Err(anyhow!("Unsupported review provider '{}'", provider)),
        }
    }

    fn upload_managed(
        &self,
        repo: &GitRepo,
        provider: &str,
        selector: &ResolvedSelector,
        destination: Option<&str>,
    ) -> Result<UnifiedReviewUpload> {
        let record = managed_record(repo, selector)?;
        let metadata = selector.metadata();
        let oids = metadata
            .steps
            .iter()
            .map(|step| step.cut.clone())
            .collect::<Vec<_>>();
        let subjects = metadata
            .steps
            .iter()
            .map(|step| step.id.clone())
            .collect::<Vec<_>>();
        match provider {
            "gerrit" => {
                let mut state: GerritProviderState = provider_state(
                    &record,
                    "git-staircase.gerrit",
                    "run review create before upload",
                )?;
                if let Some(destination) = destination {
                    state.route.destination_branch = format!(
                        "refs/heads/{}",
                        destination.trim_start_matches("refs/heads/")
                    );
                    state.route.upload_ref =
                        format!("refs/for/{}", destination.trim_start_matches("refs/heads/"));
                }
                let machine = GerritStateMachine::new(ProductionTransport);
                let plan = machine.plan(
                    repo,
                    &state.route,
                    &oids,
                    &subjects,
                    Some(&state.mapping_policy),
                    Some(&state),
                    Some((&record.record_oid, &record.structure_oid)),
                )?;
                let result = machine.upload(repo, &plan, state)?;
                let next = machine.persist(repo, &record, &result.state)?;
                Ok(UnifiedReviewUpload {
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
                })
            }
            "github" => {
                let state: GitHubProviderState = provider_state(
                    &record,
                    "git-staircase.github",
                    "run review create before upload",
                )?;
                let machine = GitHubStateMachine::new(ProductionTransport);
                let plan = machine.plan(
                    &state.route,
                    &metadata.id,
                    &oids,
                    &subjects,
                    Some(&state.mapping_policy),
                    Some(&state),
                    Some((&record.record_oid, &record.structure_oid)),
                )?;
                let result = machine.publish(repo, &plan, state, true)?;
                let next = machine.persist(repo, &record, &result.state)?;
                Ok(UnifiedReviewUpload {
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
                })
            }
            _ => Err(anyhow!("Unsupported review provider '{}'", provider)),
        }
    }

    fn attach_managed(
        &self,
        repo: &GitRepo,
        provider: &str,
        selector: &ResolvedSelector,
        review: &str,
    ) -> Result<UnifiedReviewMutation> {
        let record = managed_record(repo, selector)?;
        let subject_index = selector.step_index.unwrap_or(0);
        match provider {
            "gerrit" => {
                let state: GerritProviderState = provider_state(
                    &record,
                    "git-staircase.gerrit",
                    "run review create before attaching",
                )?;
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| anyhow!("selected step has no Gerrit association"))?;
                let machine = GerritStateMachine::new(ProductionTransport);
                if review.is_empty()
                    || !review.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "~._-".contains(character)
                    })
                {
                    return Err(anyhow!("unsafe Gerrit review selector"));
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
                    return Err(anyhow!(
                        "Gerrit attachment validation failed or is uncertain"
                    ));
                }
                let remote = parse_gerrit_api_change(&response.observations)
                    .ok_or_else(|| anyhow!("Gerrit returned malformed change metadata"))?;
                let state = machine.attach(state, &subject_id, remote)?;
                let next = machine.persist(repo, &record, &state)?;
                Ok(UnifiedReviewMutation {
                    provider_label: "Gerrit".into(),
                    action: "attach".into(),
                    changed: 1,
                    record_before: Some(record.record_oid),
                    record_after: Some(next.record_oid),
                    details: vec![format!("validated and attached Gerrit review {}", review)],
                })
            }
            "github" => {
                let state: GitHubProviderState = provider_state(
                    &record,
                    "git-staircase.github",
                    "run review create before attaching",
                )?;
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .or_else(|| state.associations.first())
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| anyhow!("selected step has no GitHub association"))?;
                let (repository, number) = parse_github_review_selector(review)?;
                if repository != state.route.base_repository.full_name() {
                    return Err(anyhow!(
                        "GitHub review selector base repository does not match route"
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
                    return Err(anyhow!(
                        "GitHub attachment validation failed or is uncertain"
                    ));
                }
                let remote =
                    parse_github_api_pull(&state.route.installation, &response.observations)
                        .ok_or_else(|| {
                            anyhow!("GitHub returned malformed pull-request metadata")
                        })?;
                let state = machine.attach(state, &subject_id, remote)?;
                let next = machine.persist(repo, &record, &state)?;
                Ok(UnifiedReviewMutation {
                    provider_label: "GitHub".into(),
                    action: "attach".into(),
                    changed: 1,
                    record_before: Some(record.record_oid),
                    record_after: Some(next.record_oid),
                    details: vec![format!("validated and attached GitHub review {}", review)],
                })
            }
            _ => Err(anyhow!("Unsupported review provider '{}'", provider)),
        }
    }

    fn detach_managed(
        &self,
        repo: &GitRepo,
        provider: &str,
        selector: &ResolvedSelector,
        review: &str,
    ) -> Result<UnifiedReviewMutation> {
        let record = managed_record(repo, selector)?;
        let subject_index = selector.step_index.unwrap_or(0);
        match provider {
            "gerrit" => {
                let state: GerritProviderState = provider_state(
                    &record,
                    "git-staircase.gerrit",
                    "no Gerrit associations exist",
                )?;
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| anyhow!("selected step has no Gerrit association"))?;
                let machine = GerritStateMachine::new(ProductionTransport);
                let state = machine.detach(state, &subject_id)?;
                let next = machine.persist(repo, &record, &state)?;
                Ok(detach_result(
                    "Gerrit",
                    review,
                    record.record_oid,
                    next.record_oid,
                ))
            }
            "github" => {
                let state: GitHubProviderState = provider_state(
                    &record,
                    "git-staircase.github",
                    "no GitHub associations exist",
                )?;
                let subject_id = state
                    .associations
                    .get(subject_index)
                    .or_else(|| state.associations.first())
                    .map(|association| association.subject_id.clone())
                    .ok_or_else(|| anyhow!("selected step has no GitHub association"))?;
                let machine = GitHubStateMachine::new(ProductionTransport);
                let state = machine.detach(state, &subject_id)?;
                let next = machine.persist(repo, &record, &state)?;
                Ok(detach_result(
                    "GitHub",
                    review,
                    record.record_oid,
                    next.record_oid,
                ))
            }
            _ => Err(anyhow!("Unsupported review provider '{}'", provider)),
        }
    }
}

fn managed_record(
    repo: &GitRepo,
    selector: &ResolvedSelector,
) -> Result<crate::model::StaircaseRecord> {
    let reference = format!("refs/staircase-state/{}/record", selector.metadata().id);
    Ok(read_record(repo, &reference)?)
}

fn provider_state<T: serde::de::DeserializeOwned>(
    record: &crate::model::StaircaseRecord,
    extension: &str,
    missing: &str,
) -> Result<T> {
    let value = record
        .user_metadata
        .extensions
        .get(extension)
        .cloned()
        .ok_or_else(|| anyhow!("{}", missing))?;
    Ok(serde_json::from_value(value)?)
}

fn detach_result(
    provider: &str,
    review: &str,
    before: String,
    after: String,
) -> UnifiedReviewMutation {
    UnifiedReviewMutation {
        provider_label: provider.into(),
        action: "detach".into(),
        changed: 1,
        record_before: Some(before),
        record_after: Some(after),
        details: vec![format!(
            "{} retained as historical association; no remote mutation performed",
            review
        )],
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
                return Err(anyhow!(
                    "GitHub selector must be installation/owner/repository#number or owner/repository#number"
                ));
            }
        };
        if repository.split('/').count() != 2 {
            return Err(anyhow!(
                "GitHub selector must be installation/owner/repository#number or owner/repository#number"
            ));
        }
        return Ok((repository, number.parse()?));
    }
    if let Some((prefix, number)) = selector.rsplit_once("/pull/") {
        let repository = prefix
            .strip_prefix("https://")
            .or_else(|| prefix.strip_prefix("http://"))
            .unwrap_or(prefix)
            .split_once('/')
            .map(|(_, repository)| repository)
            .unwrap_or(prefix);
        if repository.split('/').count() == 2 {
            return Ok((repository.into(), number.parse()?));
        }
    }
    Err(anyhow!("invalid GitHub pull-request selector"))
}

impl ToHuman for UnifiedReviewShow {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("{} Host: {}", self.provider_label, self.host));
        lines.push(format!("Project: {}", self.project));
        lines.push(format!("Destination Branch: {}", self.destination_branch));
        for (k, v) in &self.details {
            lines.push(format!("{}: {}", k, v));
        }
        lines.push("Commits:".to_string());
        for item in &self.items {
            lines.push(format!(
                "  {} {} [{}]",
                &item.oid[..7.min(item.oid.len())],
                item.title,
                item.detail
            ));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewShow {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("host\t{}", self.host));
        lines.push(format!("project\t{}", self.project));
        for item in &self.items {
            lines.push(format!("commit\t{}\t{}", item.oid, item.detail));
        }
        lines.join("\n")
    }
}

impl ToHuman for UnifiedReviewStatus {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "{} Review Status: {}",
            self.provider_label, self.status
        ));
        lines.push(format!("Host: {}", self.host));
        lines.push(format!("Project: {}", self.project));
        for (k, v) in &self.details {
            lines.push(format!("{}: {}", k, v));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewStatus {
    fn to_porcelain(&self) -> String {
        format!("status\t{}", self.status)
    }
}

impl ToHuman for UnifiedReviewPlan {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("{} Upload Plan:", self.provider_label));
        lines.push(format!("  Target Ref: {}", self.target));
        lines.push(format!("  Mapping Policy: {}", self.policy));
        lines.push("  Commits to push:".to_string());
        for item in &self.items {
            lines.push(format!(
                "    - {} {} ({})",
                &item.oid[..7.min(item.oid.len())],
                item.title,
                item.detail
            ));
        }
        if !self.warnings.is_empty() {
            lines.push("  Warnings:".to_string());
            for w in &self.warnings {
                lines.push(format!("    - {}", w));
            }
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewPlan {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("push_ref\t{}", self.target));
        lines.push(format!("mapping_policy\t{}", self.policy));
        for item in &self.items {
            lines.push(format!("commit\t{}\t{}", item.oid, item.detail));
        }
        lines.join("\n")
    }
}

impl ToHuman for UnifiedReviewUpload {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("{} Upload Complete:", self.provider_label));
        lines.push(format!("  {}", self.summary));
        for detail in &self.details {
            lines.push(format!("  {}", detail));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewUpload {
    fn to_porcelain(&self) -> String {
        format!("result\t{}", self.summary)
    }
}

impl ToHuman for UnifiedReviewReconcile {
    fn to_human(&self) -> String {
        format!("{} Reconcile Status: {}", self.provider_label, self.status)
    }
}

impl ToPorcelain for UnifiedReviewReconcile {
    fn to_porcelain(&self) -> String {
        format!("status\t{}", self.status)
    }
}

impl ToHuman for UnifiedReviewOpen {
    fn to_human(&self) -> String {
        format!("{} Review URL: {}", self.provider_label, self.url)
    }
}

impl ToPorcelain for UnifiedReviewOpen {
    fn to_porcelain(&self) -> String {
        format!("url\t{}", self.url)
    }
}

impl ToHuman for UnifiedReviewMutation {
    fn to_human(&self) -> String {
        let mut lines = vec![format!(
            "{} review {}: {} association(s)",
            self.provider_label, self.action, self.changed
        )];
        lines.extend(self.details.iter().map(|detail| format!("  {}", detail)));
        if let (Some(before), Some(after)) = (&self.record_before, &self.record_after) {
            lines.push(format!("record revision: {} -> {}", before, after));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewMutation {
    fn to_porcelain(&self) -> String {
        format!("{}\t{}\t{}", self.action, self.changed, self.provider_label)
    }
}

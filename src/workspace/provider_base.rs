use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::StaircaseRecord;
use crate::workspace::review_provider::{
    OperationJournal, ProviderTransport, SynchronizationState, TransportRequest, TransportResponse,
    UnifiedProviderLanding, UnifiedProviderVerification, UnifiedReviewMutation,
    publish_provider_extension_cas,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn persist_provider_state<S: Serialize>(
    repo: &GitRepo,
    record: &StaircaseRecord,
    extension: &str,
    state: &S,
) -> Result<StaircaseRecord> {
    publish_provider_extension_cas(repo, record, extension, serde_json::to_value(state)?)
}

pub trait ProviderAssociation {
    fn local_oid(&self) -> &str;
    fn is_retired(&self) -> bool;
    fn synchronization(&self) -> SynchronizationState;
}

pub fn verify_provider_common<S, A, V>(
    oids: &[String],
    state: &S,
    provider_label: &str,
    get_associations: impl Fn(&S) -> &[A],
    get_verification: impl Fn(&S) -> &[V],
    is_verified: impl Fn(&V, &A) -> bool,
) -> UnifiedProviderVerification
where
    A: ProviderAssociation,
{
    let mut exact = Vec::new();
    let mut stale = Vec::new();
    for association in get_associations(state) {
        if !association.is_retired() && oids.contains(&association.local_oid().to_string()) {
            if get_verification(state)
                .iter()
                .any(|v| is_verified(v, association))
            {
                exact.push(association.local_oid().to_string());
            } else {
                stale.push(association.local_oid().to_string());
            }
        }
    }
    UnifiedProviderVerification {
        provider_label: provider_label.into(),
        status: if stale.is_empty() {
            "verified"
        } else {
            "stale"
        }
        .into(),
        exact_revisions: exact,
        stale_revisions: stale,
        details: HashMap::new(),
    }
}

pub fn execute_and_journal<T: ProviderTransport>(
    repo: &GitRepo,
    transport: &T,
    provider: &str,
    operation: &str,
    expected_record_oid: Option<String>,
    request: TransportRequest,
) -> Result<TransportResponse> {
    match transport.execute(repo, &request) {
        Ok(response) => {
            if response.uncertain {
                OperationJournal::for_repo(repo)?.record(
                    provider,
                    operation,
                    expected_record_oid,
                    request,
                    response.observations.clone(),
                )?;
            }
            Ok(response)
        }
        Err(error) => {
            OperationJournal::for_repo(repo)?.record(
                provider,
                operation,
                expected_record_oid,
                request,
                serde_json::json!({ "error": error.to_string() }),
            )?;
            Err(error)
        }
    }
}

pub fn status_from_record<S: for<'de> Deserialize<'de>>(
    record: Option<&StaircaseRecord>,
    extension: &str,
) -> Result<Option<S>> {
    if let Some(record) = record {
        if let Some(value) = record.user_metadata.extensions.get(extension) {
            return Ok(Some(serde_json::from_value(value.clone())?));
        }
    }
    Ok(None)
}

pub fn land_loop_common<T, I>(
    repo: &GitRepo,
    transport: &T,
    provider: &str,
    provider_label: &str,
    mode: &str,
    targets: Vec<I>,
    create_request: impl Fn(&I) -> Result<TransportRequest>,
    get_target_id: impl Fn(&I) -> String,
    reconciliation_msg: &str,
) -> Result<UnifiedProviderLanding>
where
    T: ProviderTransport,
{
    let mut landed = Vec::new();
    for target in targets {
        let request = create_request(&target)?;
        let target_id = get_target_id(&target);
        let response =
            match execute_and_journal(repo, transport, provider, "landing", None, request) {
                Ok(response) => response,
                Err(_) => {
                    return Ok(UnifiedProviderLanding {
                        provider_label: provider_label.into(),
                        mode: mode.into(),
                        status: "landing-unknown".into(),
                        landed,
                        blocked: vec![target_id],
                        destination_oid: None,
                        details: vec![reconciliation_msg.into()],
                    });
                }
            };
        if !response.success || response.uncertain {
            return Ok(UnifiedProviderLanding {
                provider_label: provider_label.into(),
                mode: mode.into(),
                status: if response.uncertain {
                    "landing-unknown".into()
                } else {
                    "partial".into()
                },
                landed,
                blocked: vec![target_id],
                destination_oid: None,
                details: vec![reconciliation_msg.into()],
            });
        }
        landed.push(target_id);
    }
    Ok(UnifiedProviderLanding {
        provider_label: provider_label.into(),
        mode: mode.into(),
        status: "landed".into(),
        landed,
        blocked: Vec::new(),
        destination_oid: None,
        details: Vec::new(),
    })
}

pub trait ReviewStateMachine {
    type State: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync;
    type Plan: crate::workspace::review_provider::ReviewOperationPlan;
    type Route: Clone + Send + Sync;

    fn plan(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        lineage_id: &str,
        oids: &[String],
        subjects: &[String],
        mapping: Option<&str>,
        existing: Option<&Self::State>,
        record_revision: Option<(&str, &str)>,
    ) -> Result<Self::Plan>;

    fn prepare(&self, plan: &Self::Plan, existing: Option<Self::State>) -> Result<Self::State>;

    fn perform_mutation(
        &self,
        repo: &GitRepo,
        plan: &Self::Plan,
        state: Self::State,
        existing: Option<Self::State>,
    ) -> Result<(Self::State, usize, Vec<String>)>;

    fn attach(
        &self,
        repo: &GitRepo,
        state: Self::State,
        subject_id: &str,
        review: &str,
    ) -> Result<Self::State>;

    fn detach(&self, repo: &GitRepo, state: Self::State, subject_id: &str) -> Result<Self::State>;

    fn persist(
        &self,
        repo: &GitRepo,
        record: &StaircaseRecord,
        state: &Self::State,
    ) -> Result<StaircaseRecord>;
}

pub fn create_mutation_common<T>(
    repo: &GitRepo,
    oids: &[String],
    mapping: Option<&str>,
    record: Option<&StaircaseRecord>,
    provider_label: &str,
    extension_key: &str,
    probe_route: impl FnOnce(
        &GitRepo,
        Option<&crate::workspace::model::WorkspaceRecord>,
    ) -> Result<Option<T::Route>>,
    machine: T,
) -> Result<UnifiedReviewMutation>
where
    T: ReviewStateMachine,
{
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
        let route = probe_route(repo, Some(&workspace.record))?.ok_or_else(|| {
            StaircaseError::Other(format!("{} route is incomplete", provider_label))
        })?;
        let existing = record
            .user_metadata
            .extensions
            .get(extension_key)
            .cloned()
            .map(serde_json::from_value::<T::State>)
            .transpose()?;
        let plan = machine.plan(
            repo,
            &route,
            &metadata.id,
            oids,
            &subjects,
            mapping,
            existing.as_ref(),
            Some((&record.record_oid, &record.structure_oid)),
        )?;
        let state = machine.prepare(&plan, existing.clone())?;
        let (state, changed, details) = machine.perform_mutation(repo, &plan, state, existing)?;
        let next = machine.persist(repo, &record, &state)?;
        return Ok(UnifiedReviewMutation {
            provider_label: provider_label.into(),
            action: "create".into(),
            changed,
            record_before: Some(record.record_oid.clone()),
            record_after: Some(next.record_oid),
            details,
        });
    }

    let subject_ids: Vec<String> = (0..oids.len())
        .map(|index| format!("commit-{}", index + 1))
        .collect();
    let route = probe_route(repo, None)?
        .ok_or_else(|| StaircaseError::Other(format!("{} route is incomplete", provider_label)))?;
    let plan = machine.plan(
        repo,
        &route,
        "implicit",
        oids,
        &subject_ids,
        mapping,
        None,
        None,
    )?;
    let state = machine.prepare(&plan, None)?;
    let (_state, changed, details) = machine.perform_mutation(repo, &plan, state, None)?;
    Ok(UnifiedReviewMutation {
        provider_label: provider_label.into(),
        action: "create".into(),
        changed,
        record_before: None,
        record_after: None,
        details,
    })
}

pub fn attach_mutation_common<T>(
    repo: &GitRepo,
    oids: &[String],
    review: &str,
    record: Option<&StaircaseRecord>,
    selected_index: Option<usize>,
    provider_label: &str,
    extension_key: &str,
    probe_route: impl FnOnce(
        &GitRepo,
        Option<&crate::workspace::model::WorkspaceRecord>,
    ) -> Result<Option<T::Route>>,
    machine: T,
) -> Result<UnifiedReviewMutation>
where
    T: ReviewStateMachine,
{
    if let Some(record) = record {
        let selected_index = selected_index.ok_or_else(|| {
            StaircaseError::Other(format!(
                "{} attach-to-review requires an explicit step selection",
                provider_label
            ))
        })?;
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
        let route = probe_route(repo, Some(&workspace.record))?.ok_or_else(|| {
            StaircaseError::Other(format!("{} route is incomplete", provider_label))
        })?;
        let existing = record
            .user_metadata
            .extensions
            .get(extension_key)
            .cloned()
            .map(serde_json::from_value::<T::State>)
            .transpose()?;
        let plan = machine.plan(
            repo,
            &route,
            &metadata.id,
            oids,
            &subjects,
            None,
            existing.as_ref(),
            Some((&record.record_oid, &record.structure_oid)),
        )?;
        let state = machine.prepare(&plan, existing)?;
        let state = machine.attach(repo, state, &subjects[selected_index], review)?;
        let next = machine.persist(repo, &record, &state)?;
        return Ok(UnifiedReviewMutation {
            provider_label: provider_label.into(),
            action: "attach".into(),
            changed: 1,
            record_before: Some(record.record_oid.clone()),
            record_after: Some(next.record_oid),
            details: vec![format!(
                "attached subject {} to review {}",
                subjects[selected_index], review
            )],
        });
    }
    Err(StaircaseError::Other(format!(
        "{} attach-to-review is only supported for managed staircases",
        provider_label
    )))
}

pub fn detach_mutation_common<T>(
    repo: &GitRepo,
    oids: &[String],
    review: &str,
    record: Option<&StaircaseRecord>,
    selected_index: Option<usize>,
    provider_label: &str,
    extension_key: &str,
    probe_route: impl FnOnce(
        &GitRepo,
        Option<&crate::workspace::model::WorkspaceRecord>,
    ) -> Result<Option<T::Route>>,
    machine: T,
) -> Result<UnifiedReviewMutation>
where
    T: ReviewStateMachine,
{
    if let Some(record) = record {
        let selected_index = selected_index.ok_or_else(|| {
            StaircaseError::Other(format!(
                "{} detach-from-review requires an explicit step selection",
                provider_label
            ))
        })?;
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
        let route = probe_route(repo, Some(&workspace.record))?.ok_or_else(|| {
            StaircaseError::Other(format!("{} route is incomplete", provider_label))
        })?;
        let existing = record
            .user_metadata
            .extensions
            .get(extension_key)
            .cloned()
            .map(serde_json::from_value::<T::State>)
            .transpose()?;
        let plan = machine.plan(
            repo,
            &route,
            &metadata.id,
            oids,
            &subjects,
            None,
            existing.as_ref(),
            Some((&record.record_oid, &record.structure_oid)),
        )?;
        let state = machine.prepare(&plan, existing)?;
        let state = machine.detach(repo, state, &subjects[selected_index])?;
        let next = machine.persist(repo, &record, &state)?;
        return Ok(UnifiedReviewMutation {
            provider_label: provider_label.into(),
            action: "detach".into(),
            changed: 1,
            record_before: Some(record.record_oid.clone()),
            record_after: Some(next.record_oid),
            details: vec![format!(
                "detached subject {} from review {}",
                subjects[selected_index], review
            )],
        });
    }
    Err(StaircaseError::Other(format!(
        "{} detach-from-review is only supported for managed staircases",
        provider_label
    )))
}

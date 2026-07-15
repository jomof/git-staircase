use crate::error::Result;
use crate::git::GitRepo;
use crate::model::StaircaseRecord;
use crate::workspace::review_provider::{
    OperationJournal, ProviderTransport, SynchronizationState, TransportRequest, TransportResponse,
    UnifiedProviderLanding, UnifiedProviderVerification, publish_provider_extension_cas,
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
) -> Result<(TransportResponse, Option<String>)> {
    match transport.execute(repo, &request) {
        Ok(response) => {
            let mut operation_id = None;
            if response.uncertain {
                operation_id = Some(
                    OperationJournal::for_repo(repo)?
                        .record(
                            provider,
                            operation,
                            expected_record_oid,
                            request,
                            response.observations.clone(),
                        )?
                        .operation_id,
                );
            }
            Ok((response, operation_id))
        }
        Err(error) => {
            let _entry = OperationJournal::for_repo(repo)?.record(
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
        let (response, _) =
            match execute_and_journal(repo, transport, provider, "landing", None, request) {
                Ok(res) => res,
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

pub struct ReviewMutationResult<S> {
    pub state: S,
    pub status: String,
    pub accepted: usize,
    pub rejected: usize,
    pub unknown: usize,
    pub journal_operation_id: Option<String>,
}

pub struct ReviewStateMachine<T: ProviderTransport> {
    pub transport: T,
    pub provider: String,
}

impl<T: ProviderTransport> ReviewStateMachine<T> {
    pub fn new(transport: T, provider: String) -> Self {
        Self {
            transport,
            provider,
        }
    }

    pub fn execute_with_journal(
        &self,
        repo: &GitRepo,
        operation: &str,
        expected_record_oid: Option<String>,
        request: TransportRequest,
    ) -> Result<(TransportResponse, Option<String>)> {
        execute_and_journal(
            repo,
            &self.transport,
            &self.provider,
            operation,
            expected_record_oid,
            request,
        )
    }

    pub fn upload_common<S, P, I>(
        &self,
        repo: &GitRepo,
        plan: &P,
        mut state: S,
        expected_record_oid: Option<String>,
        create_request: impl Fn(&P) -> Result<TransportRequest>,
        get_items: impl Fn(&P) -> &[I],
        mark_all_unknown: impl Fn(&mut S),
        apply_responses: impl Fn(&mut S, &P, &TransportResponse) -> Result<ReviewMutationResult<S>>,
    ) -> Result<ReviewMutationResult<S>> {
        let request = create_request(plan)?;
        let (response, operation_id) = match self.execute_with_journal(
            repo,
            "upload",
            expected_record_oid.clone(),
            request.clone(),
        ) {
            Ok(res) => res,
            Err(error) => {
                mark_all_unknown(&mut state);
                let journal = OperationJournal::for_repo(repo)?;
                let entry = journal.record(
                    &self.provider,
                    "upload",
                    expected_record_oid,
                    request,
                    serde_json::json!({"error": error.to_string()}),
                )?;
                return Ok(ReviewMutationResult {
                    unknown: get_items(plan).len(),
                    state,
                    status: "upload-unknown".into(),
                    accepted: 0,
                    rejected: 0,
                    journal_operation_id: Some(entry.operation_id),
                });
            }
        };

        if response.uncertain {
            mark_all_unknown(&mut state);
            return Ok(ReviewMutationResult {
                unknown: get_items(plan).len(),
                state,
                status: "upload-unknown".into(),
                accepted: 0,
                rejected: 0,
                journal_operation_id: operation_id,
            });
        }

        if !response.success {
            return Ok(ReviewMutationResult {
                state,
                status: "rejected".into(),
                accepted: 0,
                rejected: get_items(plan).len(),
                unknown: 0,
                journal_operation_id: None,
            });
        }

        apply_responses(&mut state, plan, &response)
    }
}

use crate::error::Result;
use crate::git::GitRepo;
use crate::model::StaircaseRecord;
use crate::workspace::provider_base;
use crate::workspace::review_provider::{
    ProviderTransport, UnifiedReviewReconcile, UnifiedReviewUpload,
};
use serde::{Deserialize, Serialize};

pub struct MutationResult<S> {
    pub state: S,
    pub status: String,
    pub accepted: usize,
    pub rejected: usize,
    pub unknown: usize,
    pub journal_operation_id: Option<String>,
}

pub trait ReviewStateMachine<T: ProviderTransport>: Send + Sync {
    type State: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync;
    type Plan: Send + Sync;

    fn provider_name(&self) -> &'static str;
    fn extension_name(&self) -> &'static str;
    fn transport(&self) -> &T;

    fn plan(
        &self,
        repo: &GitRepo,
        oids: &[String],
        state: Option<&Self::State>,
        record: Option<&StaircaseRecord>,
    ) -> Result<Self::Plan>;

    fn execute_upload(
        &self,
        repo: &GitRepo,
        plan: &Self::Plan,
        state: Self::State,
    ) -> Result<MutationResult<Self::State>>;

    fn execute_reconcile(&self, repo: &GitRepo, state: Self::State) -> Result<Self::State>;

    fn persist(
        &self,
        repo: &GitRepo,
        record: &StaircaseRecord,
        state: &Self::State,
    ) -> Result<StaircaseRecord> {
        provider_base::persist_provider_state(repo, record, self.extension_name(), state)
    }

    fn upload_orchestration(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
        update_state: impl FnOnce(&mut Self::State) -> Result<()>,
        create_initial_state: impl FnOnce(&Self::Plan) -> Result<Self::State>,
    ) -> Result<UnifiedReviewUpload> {
        let (state, plan) = if let Some(record) = record {
            if let Some(value) = record.user_metadata.extensions.get(self.extension_name()) {
                let mut state: Self::State = serde_json::from_value(value.clone())?;
                update_state(&mut state)?;
                let plan = self.plan(repo, oids, Some(&state), Some(record))?;
                (state, plan)
            } else {
                let plan = self.plan(repo, oids, None, None)?;
                let state = create_initial_state(&plan)?;
                (state, plan)
            }
        } else {
            let plan = self.plan(repo, oids, None, None)?;
            let state = create_initial_state(&plan)?;
            (state, plan)
        };

        let result = self.execute_upload(repo, &plan, state)?;

        let details = if let Some(record) = record {
            let next = self.persist(repo, record, &result.state)?;
            vec![
                format!("accepted: {}", result.accepted),
                format!("rejected: {}", result.rejected),
                format!("unknown: {}", result.unknown),
                format!(
                    "record revision: {} -> {}",
                    record.record_oid, next.record_oid
                ),
            ]
        } else {
            vec![
                format!("accepted: {}", result.accepted),
                format!("unknown: {}", result.unknown),
            ]
        };

        Ok(UnifiedReviewUpload {
            provider_label: self.provider_name().to_string(),
            summary: result.status,
            details,
        })
    }

    fn reconcile_orchestration(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewReconcile> {
        if let Some(record) = record {
            if let Some(value) = record.user_metadata.extensions.get(self.extension_name()) {
                let state: Self::State = serde_json::from_value(value.clone())?;
                let state = self.execute_reconcile(repo, state)?;
                let next = self.persist(repo, record, &state)?;
                return Ok(UnifiedReviewReconcile {
                    provider_label: self.provider_name().to_string(),
                    status: format!(
                        "reconciled; record {} -> {}",
                        record.record_oid, next.record_oid
                    ),
                });
            }
        }
        let _plan = self.plan(repo, oids, None, None)?;
        Ok(UnifiedReviewReconcile {
            provider_label: self.provider_name().to_string(),
            status: format!("Reconciled with {} server", self.provider_name()),
        })
    }
}

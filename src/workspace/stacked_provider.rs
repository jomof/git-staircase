use crate::error::Result;
use crate::git::GitRepo;
use crate::model::StaircaseRecord;
use crate::workspace::provider_base;
use crate::workspace::review_provider::{
    SynchronizationState, UnifiedProviderLanding, UnifiedProviderVerification,
    UnifiedReviewMutation, UnifiedReviewOpen, UnifiedReviewPlan, UnifiedReviewReconcile,
    UnifiedReviewShow, UnifiedReviewStatus, UnifiedReviewUpload,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait StackedReviewItem: Send + Sync {
    fn subject_id(&self) -> &str;
    fn local_oid(&self) -> &str;
    fn title(&self) -> &str;
    fn detail(&self) -> String;
}

pub trait StackedReviewPlan: Send + Sync {
    type Item: StackedReviewItem;
    fn items(&self) -> &[Self::Item];
    fn mapping_policy(&self) -> &str;
    fn warnings(&self) -> &[String];
}

pub trait StackedReviewAssociation: Send + Sync {
    fn subject_id(&self) -> &str;
    fn local_oid(&self) -> &str;
    fn synchronization(&self) -> SynchronizationState;
    fn is_retired(&self) -> bool;
}

pub trait StackedReviewState: Send + Sync {
    type Association: StackedReviewAssociation;
    fn associations(&self) -> &[Self::Association];
    fn reconciliation_required(&self) -> bool;
}

pub trait StackedReviewImplementation: Send + Sync {
    type State: StackedReviewState + for<'de> Deserialize<'de> + Serialize;
    type Route;
    type Plan: StackedReviewPlan;

    fn provider_label(&self) -> &'static str;
    fn extension_name(&self) -> &'static str;

    fn create_plan(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        oids: &[String],
        mapping: Option<&str>,
    ) -> Result<Self::Plan>;

    fn get_host(&self, route: &Self::Route) -> String;
    fn get_project(&self, route: &Self::Route) -> String;
    fn get_destination_branch(&self, route: &Self::Route) -> String;
    fn get_open_url(&self, route: &Self::Route) -> String;

    fn render_association_detail(
        &self,
        association: &<Self::State as StackedReviewState>::Association,
    ) -> String;
    fn render_state_details(&self, state: &Self::State) -> HashMap<String, String>;

    fn upload(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        oids: &[String],
        destination: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewUpload>;
    fn reconcile(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewReconcile>;
    fn create(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        oids: &[String],
        mapping: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewMutation>;
    fn attach(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation>;
    fn detach(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation>;
    fn land(
        &self,
        repo: &GitRepo,
        route: &Self::Route,
        oids: &[String],
        mode: &str,
        method: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderLanding>;
}

pub struct StackedReviewInstance<I: StackedReviewImplementation> {
    pub implementation: I,
    pub route: I::Route,
}

impl<I: StackedReviewImplementation> crate::workspace::review_provider::ReviewProviderInstance
    for StackedReviewInstance<I>
{
    fn show(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewShow> {
        let plan = self
            .implementation
            .create_plan(repo, &self.route, oids, None)?;
        let items = plan
            .items()
            .iter()
            .map(
                |item| crate::workspace::review_provider::UnifiedReviewItem {
                    oid: item.local_oid().to_string(),
                    title: item.title().to_string(),
                    detail: item.detail(),
                },
            )
            .collect();

        let mut details = HashMap::new();
        details.insert(
            "Mapping Policy".to_string(),
            plan.mapping_policy().to_string(),
        );

        Ok(UnifiedReviewShow {
            provider_label: self.implementation.provider_label().to_string(),
            host: self.implementation.get_host(&self.route),
            project: self.implementation.get_project(&self.route),
            destination_branch: self.implementation.get_destination_branch(&self.route),
            details,
            items,
        })
    }

    fn status(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewStatus> {
        if let Some(state) = provider_base::status_from_record::<I::State>(
            record,
            self.implementation.extension_name(),
        )? {
            let status = if state.reconciliation_required() {
                "reconciliation-required".into()
            } else if state
                .associations()
                .iter()
                .all(|a| a.synchronization() == SynchronizationState::Current)
            {
                "current".into()
            } else {
                "pending".into()
            };

            return Ok(UnifiedReviewStatus {
                provider_label: self.implementation.provider_label().to_string(),
                status,
                host: self.implementation.get_host(&self.route),
                project: self.implementation.get_project(&self.route),
                details: self.implementation.render_state_details(&state),
            });
        }

        let plan = self
            .implementation
            .create_plan(repo, &self.route, oids, None)?;
        let mut details = HashMap::new();
        details.insert(
            "Mapping Policy".to_string(),
            plan.mapping_policy().to_string(),
        );

        Ok(UnifiedReviewStatus {
            provider_label: self.implementation.provider_label().to_string(),
            status: "unknown".into(),
            host: self.implementation.get_host(&self.route),
            project: self.implementation.get_project(&self.route),
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
        let plan = self
            .implementation
            .create_plan(repo, &self.route, oids, mapping)?;
        let items = plan
            .items()
            .iter()
            .map(
                |item| crate::workspace::review_provider::UnifiedReviewItem {
                    oid: item.local_oid().to_string(),
                    title: item.title().to_string(),
                    detail: item.detail(),
                },
            )
            .collect();

        Ok(UnifiedReviewPlan {
            provider_label: self.implementation.provider_label().to_string(),
            symbolic_integration_target: self.implementation.get_project(&self.route),
            policy: plan.mapping_policy().to_string(),
            items,
            warnings: plan.warnings().to_vec(),
        })
    }

    fn upload(
        &self,
        repo: &GitRepo,
        oids: &[String],
        destination: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewUpload> {
        self.implementation
            .upload(repo, &self.route, oids, destination, record)
    }

    fn reconcile(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewReconcile> {
        self.implementation
            .reconcile(repo, &self.route, oids, record)
    }

    fn get_stable_identifiers(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<Vec<Option<String>>> {
        let plan = self
            .implementation
            .create_plan(repo, &self.route, oids, None)?;
        Ok(plan.items().iter().map(|_| None).collect())
    }

    fn open(
        &self,
        _repo: &GitRepo,
        _oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewOpen> {
        Ok(UnifiedReviewOpen {
            provider_label: self.implementation.provider_label().to_string(),
            url: self.implementation.get_open_url(&self.route),
        })
    }

    fn create(
        &self,
        repo: &GitRepo,
        oids: &[String],
        mapping: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewMutation> {
        self.implementation
            .create(repo, &self.route, oids, mapping, record)
    }

    fn attach(
        &self,
        repo: &GitRepo,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation> {
        self.implementation
            .attach(repo, &self.route, oids, review, record, selected_index)
    }

    fn detach(
        &self,
        repo: &GitRepo,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation> {
        self.implementation
            .detach(repo, &self.route, oids, review, record, selected_index)
    }

    fn verify_provider(
        &self,
        repo: &GitRepo,
        oids: &[String],
        _record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderVerification> {
        let plan = self
            .implementation
            .create_plan(repo, &self.route, oids, None)?;
        Ok(UnifiedProviderVerification {
            provider_label: self.implementation.provider_label().into(),
            status: if plan.warnings().is_empty() {
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
        repo: &GitRepo,
        oids: &[String],
        mode: &str,
        method: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderLanding> {
        self.implementation
            .land(repo, &self.route, oids, mode, method, record)
    }
}

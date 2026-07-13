use crate::error::Result;
use crate::git::GitRepo;
use crate::workspace::model::WorkspaceRecord;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewItem {
    pub oid: String,
    pub title: String,
    pub detail: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewShow {
    pub provider_label: String,
    pub host: String,
    pub project: String,
    pub destination_branch: String,
    pub details: HashMap<String, String>,
    pub items: Vec<UnifiedReviewItem>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewStatus {
    pub provider_label: String,
    pub status: String,
    pub host: String,
    pub project: String,
    pub details: HashMap<String, String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewPlan {
    pub provider_label: String,
    pub target: String,
    pub policy: String,
    pub items: Vec<UnifiedReviewItem>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewUpload {
    pub provider_label: String,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewReconcile {
    pub provider_label: String,
    pub status: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewOpen {
    pub provider_label: String,
    pub url: String,
}

pub trait ReviewProvider {
    fn name(&self) -> &'static str;
    fn probe(
        &self,
        repo: &GitRepo,
        record: Option<&WorkspaceRecord>,
    ) -> Result<Option<Box<dyn ReviewProviderInstance>>>;
}

pub trait ReviewProviderInstance {
    fn show(&self, repo: &GitRepo, oids: &[String]) -> Result<UnifiedReviewShow>;
    fn status(&self, repo: &GitRepo, oids: &[String]) -> Result<UnifiedReviewStatus>;
    fn plan(
        &self,
        repo: &GitRepo,
        oids: &[String],
        mapping: Option<&str>,
    ) -> Result<UnifiedReviewPlan>;
    fn upload(
        &self,
        repo: &GitRepo,
        oids: &[String],
        destination: Option<&str>,
    ) -> Result<UnifiedReviewUpload>;
    fn reconcile(&self, repo: &GitRepo, oids: &[String]) -> Result<UnifiedReviewReconcile>;
    fn open(&self, repo: &GitRepo, oids: &[String]) -> Result<UnifiedReviewOpen>;
}

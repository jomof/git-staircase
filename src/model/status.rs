use crate::model::draft::WorktreeDraft;
use crate::model::metadata::StaircaseMetadata;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StepStatus {
    pub name: String,
    pub expected_cut: String,
    pub actual_oid: Option<String>,
    pub is_stale: bool,
    pub is_modified: bool,
    pub is_incomplete: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StaircaseState {
    Clean,
    Incomplete,
    Diverged,
    Ambiguous,
    Stale,
}

impl fmt::Display for StaircaseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StaircaseState::Clean => write!(f, "clean"),
            StaircaseState::Stale => write!(f, "stale"),
            StaircaseState::Incomplete => write!(f, "incomplete"),
            StaircaseState::Diverged => write!(f, "diverged"),
            StaircaseState::Ambiguous => write!(f, "ambiguous"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseStatus {
    pub verification_results: Option<Vec<VerificationResult>>,
    pub metadata: StaircaseMetadata,
    pub steps: Vec<StepStatus>,
    pub is_clean: bool,
    pub is_implicit: bool,
    pub is_diverged: bool,
    pub is_ambiguous: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_draft: Option<WorktreeDraft>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_operation: Option<ActiveOperationStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_git_operation: Option<ExternalGitOperationStatus>,
}

impl StaircaseStatus {
    pub fn state(&self) -> StaircaseState {
        if self.is_ambiguous {
            StaircaseState::Ambiguous
        } else if self.is_diverged {
            StaircaseState::Diverged
        } else if self.steps.iter().any(|s| s.is_incomplete) {
            StaircaseState::Incomplete
        } else if self.steps.iter().any(|s| s.is_stale) {
            StaircaseState::Stale
        } else if self.is_diverged || self.steps.iter().any(|s| s.is_modified) {
            StaircaseState::Diverged
        } else {
            StaircaseState::Clean
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ActiveOperationStatus {
    pub operation_id: String,
    pub kind: String,
    pub phase: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExternalGitOperationStatus {
    pub operation: String,
    pub owner: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
    pub step_name: String,
    pub cut: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(ValueEnum)]
pub enum IdentityKind {
    Lineage,
    Revision,
    Body,
    Decomposition,
    Outcome,
    PatchSeries,
    Nominal,
    Review,
}

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub id: String,
    pub name: String,
    pub cut: String,            // Commit OID
    pub branch: Option<String>, // Optional local branch name (ref name without refs/heads/)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VerificationPolicy {
    pub build_command: Option<String>,
    pub test_command: Option<String>,
    pub verify_each_prefix: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum LandingPolicy {
    AggregateOnly,
    Stepwise,
    Either,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseMetadata {
    pub landing_policy: Option<LandingPolicy>,
    pub id: String,     // UUID
    pub name: String,   // Nominal name
    pub target: String, // Integration boundary (e.g., "refs/remotes/origin/main" or "main")
    pub steps: Vec<Step>,
    pub verification_policy: Option<VerificationPolicy>,
    pub primary_branch_layout: Option<String>,
    pub branch_layout_base: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FamilyStep {
    pub name: String,
    pub cut: String,
    pub branch: Option<String>,
    pub children: Vec<String>, // Names of child steps
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseFamily {
    pub id: String,
    pub name: String,
    pub target: String,
    pub steps: HashMap<String, FamilyStep>,
    pub roots: Vec<String>, // Names of root steps
    pub verification_policy: Option<VerificationPolicy>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Discovery {
    Linear(StaircaseMetadata),
    Ambiguous(StaircaseFamily),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    pub refname: String, // e.g. "refs/heads/feature/auth-core"
    pub oid: String,
    pub upstream: Option<String>, // e.g. "refs/remotes/origin/main" or "refs/heads/feature/auth-core"
}

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
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RewriteMode {
    Amend,
    Fixup,
    FoldInto(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DraftIntent {
    Unassigned,
    ExtendStep,
    NewStep,
    RewriteStep(RewriteMode),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DraftClassification {
    Clean,
    StagedOnly,
    UnstagedOnly,
    PartiallyStaged,
    Untracked,
    Conflicted,
    TransientOperation,
    SubmoduleDirty,
}

impl fmt::Display for DraftClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DraftClassification::Clean => write!(f, "clean"),
            DraftClassification::StagedOnly => write!(f, "staged-only"),
            DraftClassification::UnstagedOnly => write!(f, "unstaged-only"),
            DraftClassification::PartiallyStaged => write!(f, "partially-staged"),
            DraftClassification::Untracked => write!(f, "untracked"),
            DraftClassification::Conflicted => write!(f, "conflicted"),
            DraftClassification::TransientOperation => write!(f, "transient-operation"),
            DraftClassification::SubmoduleDirty => write!(f, "submodule-dirty"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DraftAttachment {
    pub staircase_id: Option<String>,
    pub staircase_name: Option<String>,
    pub step_id: Option<String>,
    pub step_name: Option<String>,
    pub intent: DraftIntent,
    pub expected_basis: String,
    pub worktree_identity: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDraft {
    pub basis: String,
    pub head_branch: Option<String>,
    pub staged_paths: Vec<String>,
    pub staged_tree_oid: Option<String>,
    pub unstaged_paths: Vec<String>,
    pub untracked_paths: Vec<String>,
    pub ignored_paths: Vec<String>,
    pub conflicted_paths: Vec<String>,
    pub transient_operation: Option<String>,
    pub is_submodule_dirty: bool,
    pub attachment: Option<DraftAttachment>,
    pub classification: DraftClassification,
    pub is_attachment_stale: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DraftSnapshot {
    pub id: String,
    pub created_at: String,
    pub basis: String,
    pub staged_tree: Option<String>,
    pub worktree_tree: Option<String>,
    pub untracked_paths: Vec<String>,
    pub attachment: Option<DraftAttachment>,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
    pub step_name: String,
    pub cut: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

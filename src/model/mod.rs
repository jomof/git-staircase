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
    pub id: String,                          // UUID
    pub name: String,                        // Nominal name
    pub symbolic_integration_target: String, // Integration boundary (e.g., "refs/remotes/origin/main" or "main")
    pub steps: Vec<Step>,
    pub verification_policy: Option<VerificationPolicy>,
    pub primary_branch_layout: Option<String>,
    pub branch_layout_base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_metadata: Option<StaircaseUserMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<StaircaseLifecycle>,
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
    pub symbolic_integration_target: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_operation: Option<ActiveOperationStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_git_operation: Option<ExternalGitOperationStatus>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub untracked_files: Vec<DraftFileSnapshot>,
    pub attachment: Option<DraftAttachment>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DraftFileSnapshot {
    pub path: String,
    pub kind: String,
    pub mode: u32,
    pub content_hex: String,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct StaircaseLink {
    pub id: String,
    pub relationship: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct StepMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<StaircaseLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct StaircaseUserMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<StaircaseLink>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub step_metadata: HashMap<String, StepMetadata>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleState {
    Active,
    Archived,
}

impl Default for LifecycleState {
    fn default() -> Self {
        LifecycleState::Active
    }
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleState::Active => write!(f, "active"),
            LifecycleState::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LifecycleEvent {
    pub event_id: String,
    pub kind: String,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_oid_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_oid_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub details: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseLifecycle {
    pub state: LifecycleState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_reason: Option<String>,
    #[serde(default = "default_true")]
    pub name_reserved: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<LifecycleEvent>,
}

fn default_true() -> bool {
    true
}

impl Default for StaircaseLifecycle {
    fn default() -> Self {
        Self {
            state: LifecycleState::Active,
            archive_reason: None,
            name_reserved: true,
            events: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ArchivedOwnedRef {
    pub ref_id: String,
    pub original_refname: String,
    pub object_type: String,
    pub original_oid: String,
    pub archive_refname: String,
    pub ownership_class: String,
    pub visibility_class: String,
    pub restoration_policy: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BranchConfigEntry {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BranchConfigSnapshot {
    pub branch_name: String,
    pub entries: Vec<BranchConfigEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ArchiveManifest {
    pub archive_event_id: String,
    pub lineage_id: String,
    pub archive_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub previous_record_oid: String,
    pub canonical_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_layout_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_layout_base: Option<String>,
    pub owned_refs: Vec<ArchivedOwnedRef>,
    pub expected_source_oids: HashMap<String, String>,
    pub archive_retention_refs: HashMap<String, String>,
    pub branch_configs: Vec<BranchConfigSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worktree_attachments: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_disposition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_disposition: Option<String>,
    pub name_reservation: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseRecord {
    pub record_oid: String,
    pub structure_oid: String,
    pub metadata_oid: String,
    pub lifecycle_oid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_manifest_oid: Option<String>,
    pub metadata: StaircaseMetadata,
    pub user_metadata: StaircaseUserMetadata,
    pub lifecycle: StaircaseLifecycle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_manifest: Option<ArchiveManifest>,
}

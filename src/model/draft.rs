use serde::{Deserialize, Serialize};
use std::fmt;

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
    pub worktree_identity: Option<String>,
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

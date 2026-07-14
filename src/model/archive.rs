use super::metadata::StaircaseMetadata;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
    #[serde(default)]
    pub lineage_id: String,
    pub archive_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
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
    pub worktree_attachments: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_disposition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_disposition: Option<String>,
    pub name_reservation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub originating_structural_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ImplicitSnapshotDescriptor {
    pub schema_version: String,
    pub representation_kind: String,
    pub archive_id: String,
    pub originating_structural_key: String,
    pub integration_context: String,
    pub ordered_cuts: Vec<String>,
    pub step_count: usize,
    pub canonical_display_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materializing_refs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ImplicitArchiveLifecycle {
    pub state: String,
    pub archive_id: String,
    pub archive_event_id: String,
    pub archive_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub source_representation: String,
    pub name_reservation: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ImplicitArchiveSnapshot {
    pub archive_id: String,
    pub record_oid: String,
    pub descriptor: ImplicitSnapshotDescriptor,
    pub lifecycle: ImplicitArchiveLifecycle,
    pub manifest: ArchiveManifest,
    #[serde(skip, default = "empty_staircase_metadata")]
    pub metadata: StaircaseMetadata,
}

fn empty_staircase_metadata() -> StaircaseMetadata {
    StaircaseMetadata {
        landing_policy: None,
        id: String::new(),
        verification_policy: None,
        name: String::new(),
        target: String::new(),
        steps: vec![],
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    }
}

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
    pub worktree_attachments: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_disposition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_disposition: Option<String>,
    pub name_reservation: bool,
}

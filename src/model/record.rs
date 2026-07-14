use crate::model::archive::ArchiveManifest;
use crate::model::lifecycle::StaircaseLifecycle;
use crate::model::metadata::{StaircaseMetadata, StaircaseUserMetadata};
use serde::{Deserialize, Serialize};

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

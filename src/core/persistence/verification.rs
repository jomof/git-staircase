use crate::core::refs::StaircaseRefs;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{IdentityKind, VerificationResult};

pub fn record_verification(
    repo: &GitRepo,
    key: &str,
    kind: IdentityKind,
    results: &[VerificationResult],
) -> Result<String> {
    let ref_name = match kind {
        IdentityKind::Lineage => StaircaseRefs::verification(key),
        IdentityKind::Revision => StaircaseRefs::revision_verification(&key.replace(":", "/")),
        _ => {
            return Err(StaircaseError::Other(format!(
                "Unsupported identity kind for verification: {:?}",
                kind
            )));
        }
    };

    let commit_msg = format!(
        "Record verification for staircase {} (kind: {:?})",
        key, kind
    );

    super::commit_json_data(repo, &ref_name, &results, "verification.json", &commit_msg)
}

pub fn read_verification(
    repo: &GitRepo,
    key: &str,
    kind: IdentityKind,
) -> Result<Option<Vec<VerificationResult>>> {
    let ref_name = match kind {
        IdentityKind::Lineage => StaircaseRefs::verification(key),
        IdentityKind::Revision => StaircaseRefs::revision_verification(&key.replace(":", "/")),
        _ => {
            return Err(StaircaseError::Other(format!(
                "Unsupported identity kind for verification: {:?}",
                kind
            )));
        }
    };

    super::read_json_data_opt(repo, &ref_name, "verification.json")
}

pub mod record;
pub mod structure;
pub mod verification;

use crate::core::refs::{ARCHIVE_PREFIX, STATE_PREFIX, StaircaseRefs};
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::StaircaseMetadata;

pub use record::{
    read_metadata, read_metadata_from_oid, read_record, write_metadata, write_record,
};
pub use structure::{parse_descriptor, parse_structure, serialize_descriptor, serialize_structure};
pub use verification::{read_verification, record_verification};

pub fn list_staircases(repo: &GitRepo) -> Result<Vec<StaircaseMetadata>> {
    list_staircases_filtered(repo, true, false)
}

pub fn list_archived_staircases(repo: &GitRepo) -> Result<Vec<StaircaseMetadata>> {
    list_staircases_filtered(repo, false, true)
}

pub fn list_all_staircases(repo: &GitRepo) -> Result<Vec<StaircaseMetadata>> {
    list_staircases_filtered(repo, true, true)
}

pub fn list_staircases_filtered(
    repo: &GitRepo,
    include_active: bool,
    include_archived: bool,
) -> Result<Vec<StaircaseMetadata>> {
    let mut staircases = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    if include_active {
        for line in repo.for_each_ref(STATE_PREFIX, "%(refname) %(objectname)", None)? {
            let mut parts = line.split_whitespace();
            let _refname = parts
                .next()
                .ok_or_else(|| StaircaseError::Other("invalid for-each-ref output".into()))?;
            let oid = parts
                .next()
                .ok_or_else(|| StaircaseError::Other("invalid for-each-ref output".into()))?;
            if let Ok(metadata) = read_metadata_from_oid(repo, oid) {
                if seen_ids.insert(metadata.id.clone()) {
                    staircases.push(metadata);
                }
            } else {
                // If it's not a valid record, it might be a direct revision ref, ignore it.
            }
        }
    }

    if include_archived {
        for line in repo.for_each_ref(ARCHIVE_PREFIX, "%(refname) %(objectname)", None)? {
            let mut parts = line.split_whitespace();
            let _refname = parts
                .next()
                .ok_or_else(|| StaircaseError::Other("invalid for-each-ref output".into()))?;
            let oid = parts
                .next()
                .ok_or_else(|| StaircaseError::Other("invalid for-each-ref output".into()))?;
            if let Ok(metadata) = read_metadata_from_oid(repo, oid) {
                if seen_ids.insert(metadata.id.clone()) {
                    staircases.push(metadata);
                }
            }
        }
    }

    Ok(staircases)
}

pub fn delete_staircase_refs(repo: &GitRepo, id: &str, name: &str) -> Result<()> {
    let mut refs = vec![
        StaircaseRefs::state_record(id),
        StaircaseRefs::archive_record(id),
        StaircaseRefs::verification(id),
    ];
    if !name.is_empty() {
        refs.push(StaircaseRefs::public(name));
    }

    let mut commands = Vec::new();
    for r in refs {
        if let Some(oid) = repo.resolve_ref_opt(&r)? {
            commands.push(format!("delete {} {}", r, oid));
        }
    }

    if !commands.is_empty() {
        repo.update_refs_transaction(&commands)?;
    }
    Ok(())
}

// Internal utilities used by submodules

pub(crate) fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
    fn sort(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(sort).collect())
            }
            serde_json::Value::Object(values) => {
                let mut sorted = serde_json::Map::new();
                let mut entries: Vec<_> = values.into_iter().collect();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                for (key, value) in entries {
                    sorted.insert(key, sort(value));
                }
                serde_json::Value::Object(sorted)
            }
            scalar => scalar,
        }
    }

    let value = sort(serde_json::to_value(value)?);
    Ok(serde_json::to_string(&value)?)
}

pub(crate) fn write_versioned_json<T: serde::Serialize>(
    repo: &GitRepo,
    header: &str,
    value: &T,
) -> Result<String> {
    repo.write_blob(&format!("{}\n{}\n", header, canonical_json(value)?))
}

pub(crate) fn read_versioned_json<T: serde::de::DeserializeOwned>(
    repo: &GitRepo,
    oid: &str,
    expected_header: &str,
) -> Result<T> {
    let content = repo.cat_file(oid)?;
    let (header, json) = content
        .split_once('\n')
        .ok_or_else(|| StaircaseError::Other(format!("versioned blob {} has no header", oid)))?;
    if header != expected_header {
        return Err(StaircaseError::Other(format!(
            "blob {} has header '{}', expected '{}'",
            oid, header, expected_header
        )));
    }
    Ok(serde_json::from_str(json.trim_end_matches('\n'))?)
}

pub(crate) fn commit_json_data<T: serde::Serialize>(
    repo: &GitRepo,
    refname: &str,
    data: &T,
    filename: &str,
    message: &str,
) -> Result<String> {
    let content = format!("{}\n", canonical_json(data)?);
    let blob_oid = repo.write_blob(&content)?;
    let tree_oid = repo.write_tree(&[crate::git::TreeEntry::blob(&blob_oid, filename)])?;
    let parent = repo.resolve_ref_opt(refname)?;
    let parents = if let Some(p) = &parent {
        vec![p.as_str()]
    } else {
        vec![]
    };
    let commit_oid = repo.commit_tree(&tree_oid, &parents, message)?;
    repo.update_ref(refname, &commit_oid, parent.as_deref())?;
    Ok(commit_oid)
}

pub(crate) fn read_json_data_opt<T: serde::de::DeserializeOwned>(
    repo: &GitRepo,
    target: &str,
    filename: &str,
) -> Result<Option<T>> {
    let oid = match repo.resolve_ref_opt(target)? {
        Some(oid) => oid,
        None => return Ok(None),
    };
    let tree_oid = repo.get_tree_id(&oid)?;
    let entries = repo.ls_tree(&tree_oid)?;
    let entry = entries.into_iter().find(|e| e.name == filename);
    match entry {
        Some(e) => {
            let content = repo.cat_file(&e.oid)?;
            Ok(Some(serde_json::from_str(&content)?))
        }
        None => Ok(None),
    }
}

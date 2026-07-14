use crate::core::refs::{ARCHIVE_PREFIX, PUBLIC_PREFIX, STATE_PREFIX, StaircaseRefs};
use crate::error::{Result, StaircaseError};
use crate::git::{GitRepo, TreeEntry};
use crate::model::{
    ArchiveManifest, IdentityKind, LifecycleState, StaircaseLifecycle, StaircaseMetadata,
    StaircaseRecord, StaircaseUserMetadata, Step, VerificationResult,
};
use std::collections::BTreeMap;

const POLICY_EXTENSION: &str = "git-staircase.policies";
const DISCOVERY_EXTENSION: &str = "git-staircase.discovery-overrides";
const ANCHOR_EXTENSION: &str = "git-staircase.internal.integration-anchor";
const STRUCTURAL_STATE_EXTENSION: &str = "git-staircase.internal.structural-state";
const GERRIT_EXTENSION: &str = "git-staircase.gerrit";
const GITHUB_EXTENSION: &str = "git-staircase.github";

pub fn write_record(
    repo: &GitRepo,
    metadata: &StaircaseMetadata,
    user_metadata: &StaircaseUserMetadata,
    lifecycle: &StaircaseLifecycle,
    archive_manifest: Option<&ArchiveManifest>,
    expected_record_oid: Option<&str>,
    update_refs: bool,
) -> Result<StaircaseRecord> {
    let structure_desc = serialize_structure(repo, metadata, user_metadata)?;
    let structure_oid = repo.write_blob(&structure_desc)?;

    let mut descriptive_metadata = user_metadata.clone();
    descriptive_metadata.extensions.remove(POLICY_EXTENSION);
    descriptive_metadata.extensions.remove(DISCOVERY_EXTENSION);
    descriptive_metadata.extensions.remove(ANCHOR_EXTENSION);
    descriptive_metadata
        .extensions
        .remove(STRUCTURAL_STATE_EXTENSION);
    descriptive_metadata.extensions.remove(GERRIT_EXTENSION);
    descriptive_metadata.extensions.remove(GITHUB_EXTENSION);
    let metadata_oid =
        write_versioned_json(repo, "git-staircase-metadata 1", &descriptive_metadata)?;
    let lifecycle_oid = write_versioned_json(repo, "git-staircase-lifecycle 1", lifecycle)?;

    let mut entries = vec![
        TreeEntry::blob(&lifecycle_oid, "lifecycle"),
        TreeEntry::blob(&metadata_oid, "metadata"),
        TreeEntry::blob(&structure_oid, "structure"),
    ];

    let manifest_oid = if let Some(manifest) = archive_manifest {
        let m_oid = write_versioned_json(repo, "git-staircase-archive-manifest 1", manifest)?;
        entries.insert(0, TreeEntry::blob(&m_oid, "archive-manifest"));
        Some(m_oid)
    } else {
        None
    };

    let record_oid = repo.write_tree(&entries)?;

    let mut full_meta = metadata.clone();
    full_meta.user_metadata = Some(user_metadata.clone());
    full_meta.lifecycle = Some(lifecycle.clone());

    if update_refs {
        publish_record(repo, metadata, lifecycle, expected_record_oid, &record_oid)?;
    }

    Ok(StaircaseRecord {
        record_oid,
        structure_oid,
        metadata_oid,
        lifecycle_oid,
        archive_manifest_oid: manifest_oid,
        metadata: full_meta,
        user_metadata: user_metadata.clone(),
        lifecycle: lifecycle.clone(),
        archive_manifest: archive_manifest.cloned(),
    })
}

fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
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

fn write_versioned_json<T: serde::Serialize>(
    repo: &GitRepo,
    header: &str,
    value: &T,
) -> Result<String> {
    repo.write_blob(&format!("{}\n{}\n", header, canonical_json(value)?))
}

fn step_key(step: &Step) -> &str {
    if step.id.is_empty() {
        &step.name
    } else {
        &step.id
    }
}

fn assert_ref(repo: &GitRepo, reference: &str, expected: &str) -> Result<()> {
    let actual = repo
        .resolve_ref_opt(reference)?
        .unwrap_or_else(|| "<missing>".into());
    if actual != expected {
        return Err(StaircaseError::ConcurrentRecordUpdate {
            reference: reference.into(),
            expected: expected.into(),
            actual,
        });
    }
    Ok(())
}

fn publish_record(
    repo: &GitRepo,
    metadata: &StaircaseMetadata,
    lifecycle: &StaircaseLifecycle,
    expected_record_oid: Option<&str>,
    new_record_oid: &str,
) -> Result<()> {
    let mut old_record = expected_record_oid
        .map(|oid| read_record(repo, oid))
        .transpose()?;
    if let Some(old) = &mut old_record {
        if old.metadata.name.is_empty() {
            old.metadata.name = metadata.name.clone();
        }
    }
    let mut commands = Vec::new();

    if let Some(expected) = expected_record_oid {
        let old = old_record.as_ref().expect("expected record was read");
        let old_ref = match old.lifecycle.state {
            LifecycleState::Active => StaircaseRefs::state_record(&old.metadata.id),
            LifecycleState::Archived => StaircaseRefs::archive_record(&old.metadata.id),
        };
        assert_ref(repo, &old_ref, expected)?;
        if old.lifecycle.state == LifecycleState::Active && !old.metadata.name.is_empty() {
            assert_ref(repo, &StaircaseRefs::public(&old.metadata.name), expected)?;
        }
    }

    let new_record_ref = match lifecycle.state {
        LifecycleState::Active => StaircaseRefs::state_record(&metadata.id),
        LifecycleState::Archived => StaircaseRefs::archive_record(&metadata.id),
    };

    match old_record.as_ref() {
        None => commands.push(format!("create {} {}", new_record_ref, new_record_oid)),
        Some(old) if old.lifecycle.state == lifecycle.state => commands.push(format!(
            "update {} {} {}",
            new_record_ref, new_record_oid, old.record_oid
        )),
        Some(old) => {
            let old_ref = match old.lifecycle.state {
                LifecycleState::Active => StaircaseRefs::state_record(&old.metadata.id),
                LifecycleState::Archived => StaircaseRefs::archive_record(&old.metadata.id),
            };
            commands.push(format!("delete {} {}", old_ref, old.record_oid));
            commands.push(format!("create {} {}", new_record_ref, new_record_oid));
        }
    }

    let old_public = old_record
        .as_ref()
        .filter(|record| record.lifecycle.state == LifecycleState::Active)
        .filter(|record| !record.metadata.name.is_empty())
        .map(|record| StaircaseRefs::public(&record.metadata.name));
    let new_public = (lifecycle.state == LifecycleState::Active && !metadata.name.is_empty())
        .then(|| StaircaseRefs::public(&metadata.name));
    match (old_public.as_deref(), new_public.as_deref()) {
        (None, Some(new_ref)) => {
            commands.push(format!("create {} {}", new_ref, new_record_oid));
        }
        (Some(old_ref), None) => {
            commands.push(format!(
                "delete {} {}",
                old_ref,
                expected_record_oid.expect("old public ref has expected record")
            ));
        }
        (Some(old_ref), Some(new_ref)) if old_ref == new_ref => {
            commands.push(format!(
                "update {} {} {}",
                new_ref,
                new_record_oid,
                expected_record_oid.expect("old public ref has expected record")
            ));
        }
        (Some(old_ref), Some(new_ref)) => {
            commands.push(format!(
                "delete {} {}",
                old_ref,
                expected_record_oid.expect("old public ref has expected record")
            ));
            commands.push(format!("create {} {}", new_ref, new_record_oid));
        }
        (None, None) => {}
    }

    let old_steps = old_record
        .as_ref()
        .map(|record| {
            record
                .metadata
                .steps
                .iter()
                .map(|step| (step_key(step).to_string(), step.cut.clone()))
                .collect::<std::collections::BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let new_steps = metadata
        .steps
        .iter()
        .map(|step| (step_key(step).to_string(), step.cut.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let old_archived = old_record
        .as_ref()
        .is_some_and(|record| record.lifecycle.state == LifecycleState::Archived);
    let new_archived = lifecycle.state == LifecycleState::Archived;

    for key in old_steps.keys() {
        let old_ref = if old_archived {
            StaircaseRefs::archive_step(&metadata.id, key)
        } else {
            StaircaseRefs::state_step(&metadata.id, key)
        };
        if old_archived != new_archived || !new_steps.contains_key(key) {
            if let Some(actual) = repo.resolve_ref_opt(&old_ref)? {
                commands.push(format!("delete {} {}", old_ref, actual));
            }
        }
    }
    for (key, oid) in &new_steps {
        let new_ref = if new_archived {
            StaircaseRefs::archive_step(&metadata.id, key)
        } else {
            StaircaseRefs::state_step(&metadata.id, key)
        };
        if let Some(actual) = repo.resolve_ref_opt(&new_ref)? {
            commands.push(format!("update {} {} {}", new_ref, oid, actual));
        } else {
            commands.push(format!("create {} {}", new_ref, oid));
        }
    }

    let mut plan =
        crate::core::operation::MutationPlan::new("record-publication", Some(metadata.id.clone()))
            .expected_record(expected_record_oid.map(str::to_string));
    for command in &commands {
        let fields = command.split_whitespace().collect::<Vec<_>>();
        match fields.as_slice() {
            ["create", reference, new] => {
                plan.update((*reference).to_string(), None, Some((*new).to_string()))
            }
            ["update", reference, new, old] => plan.update(
                (*reference).to_string(),
                Some((*old).to_string()),
                Some((*new).to_string()),
            ),
            ["delete", reference, old] => {
                plan.update((*reference).to_string(), Some((*old).to_string()), None)
            }
            _ => {
                return Err(StaircaseError::Other(format!(
                    "invalid record publication command '{}'",
                    command
                )));
            }
        }
    }
    plan.publish(repo, false).map(|_| ()).map_err(|error| {
        if let Some(expected) = expected_record_oid {
            let actual = repo
                .resolve_ref_opt(&new_record_ref)
                .ok()
                .flatten()
                .unwrap_or_else(|| "<missing>".into());
            StaircaseError::ConcurrentRecordUpdate {
                reference: new_record_ref,
                expected: expected.into(),
                actual,
            }
        } else {
            error
        }
    })
}

pub fn read_record(repo: &GitRepo, target: &str) -> Result<StaircaseRecord> {
    let target_oid = repo
        .resolve_ref_opt(target)?
        .unwrap_or_else(|| target.to_string());
    let obj_type = repo.run(&["cat-file", "-t", &target_oid])?;
    if obj_type.trim() != "tree" {
        return Err(StaircaseError::Other(format!(
            "record {} must be a tree, found {}",
            target_oid,
            obj_type.trim()
        )));
    }

    let ls_output = repo.run(&["ls-tree", &target_oid])?;
    let mut entries = std::collections::BTreeMap::new();
    for line in ls_output.lines() {
        let (metadata, name) = line.split_once('\t').ok_or_else(|| {
            StaircaseError::Other(format!("invalid record tree entry in {}", target_oid))
        })?;
        let fields: Vec<_> = metadata.split_whitespace().collect();
        if fields.len() != 3 || fields[0] != "100644" || fields[1] != "blob" {
            return Err(StaircaseError::Other(format!(
                "record entry '{}' must be a regular blob",
                name
            )));
        }
        if !matches!(
            name,
            "structure" | "metadata" | "lifecycle" | "archive-manifest"
        ) {
            return Err(StaircaseError::Other(format!(
                "record tree {} contains unknown entry '{}'",
                target_oid, name
            )));
        }
        entries.insert(name.to_string(), fields[2].to_string());
    }

    let structure_oid = required_entry(&entries, "structure", &target_oid)?;
    let metadata_oid = required_entry(&entries, "metadata", &target_oid)?;
    let lifecycle_oid = required_entry(&entries, "lifecycle", &target_oid)?;
    let manifest_oid = entries.get("archive-manifest").cloned();

    let structure = repo.run(&["cat-file", "-p", &structure_oid])?;
    let (mut metadata, structural_extensions) = parse_structure(&structure)?;
    let mut user_metadata: StaircaseUserMetadata =
        read_versioned_json(repo, &metadata_oid, "git-staircase-metadata 1")?;
    user_metadata.extensions.extend(structural_extensions);
    let lifecycle: StaircaseLifecycle =
        read_versioned_json(repo, &lifecycle_oid, "git-staircase-lifecycle 1")?;
    let archive_manifest: Option<ArchiveManifest> = manifest_oid
        .as_ref()
        .map(|oid| read_versioned_json(repo, oid, "git-staircase-archive-manifest 1"))
        .transpose()?;

    if lifecycle.state == LifecycleState::Archived && archive_manifest.is_none() {
        return Err(StaircaseError::Other(format!(
            "archived record {} is missing archive-manifest",
            target_oid
        )));
    }
    if lifecycle.state == LifecycleState::Active && archive_manifest.is_some() {
        return Err(StaircaseError::Other(format!(
            "active record {} must not contain archive-manifest",
            target_oid
        )));
    }
    if let Some(manifest) = &archive_manifest {
        metadata.name = manifest.canonical_name.clone();
    }
    metadata.user_metadata = Some(user_metadata.clone());
    metadata.lifecycle = Some(lifecycle.clone());

    Ok(StaircaseRecord {
        record_oid: target_oid,
        structure_oid,
        metadata_oid,
        lifecycle_oid,
        archive_manifest_oid: manifest_oid,
        metadata,
        user_metadata,
        lifecycle,
        archive_manifest,
    })
}

fn required_entry(
    entries: &std::collections::BTreeMap<String, String>,
    name: &str,
    record_oid: &str,
) -> Result<String> {
    entries.get(name).cloned().ok_or_else(|| {
        StaircaseError::Other(format!("record tree {} missing '{}'", record_oid, name))
    })
}

fn read_versioned_json<T: serde::de::DeserializeOwned>(
    repo: &GitRepo,
    oid: &str,
    expected_header: &str,
) -> Result<T> {
    let content = repo.run(&["cat-file", "-p", oid])?;
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

pub fn write_metadata(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    let mut metadata = metadata.clone();
    for step in &mut metadata.steps {
        if step.id.is_empty() {
            step.id = uuid::Uuid::new_v4().to_string();
        }
    }
    let user_metadata = metadata.user_metadata.clone().unwrap_or_default();
    let lifecycle = metadata.lifecycle.clone().unwrap_or_default();
    let current_ref = match lifecycle.state {
        LifecycleState::Active => StaircaseRefs::state_record(&metadata.id),
        LifecycleState::Archived => StaircaseRefs::archive_record(&metadata.id),
    };
    let expected = repo.resolve_ref_opt(&current_ref)?;
    let record = write_record(
        repo,
        &metadata,
        &user_metadata,
        &lifecycle,
        None,
        expected.as_deref(),
        true,
    )?;
    Ok(record.record_oid)
}

pub fn serialize_descriptor(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    serialize_structure(repo, metadata, &StaircaseUserMetadata::default())
}

fn serialize_structure(
    repo: &GitRepo,
    metadata: &StaircaseMetadata,
    user_metadata: &StaircaseUserMetadata,
) -> Result<String> {
    let object_format = repo.get_object_format()?;
    let target_oid = user_metadata
        .extensions
        .get(ANCHOR_EXTENSION)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .map(Ok)
        .unwrap_or_else(|| repo.resolve_commit(&metadata.target))?;
    let typed_oid = |hex: String| {
        serde_json::json!({
            "algorithm": object_format,
            "hex": hex,
        })
    };
    let steps = metadata
        .steps
        .iter()
        .map(|step| {
            let materializing_refs = step
                .branch
                .as_ref()
                .map(|branch| {
                    vec![if branch.starts_with("refs/") {
                        branch.clone()
                    } else {
                        format!("refs/heads/{}", branch)
                    }]
                })
                .unwrap_or_default();
            serde_json::json!({
                "id": step.id,
                "name": step.name,
                "cut_oid": typed_oid(step.cut.clone()),
                "materializing_refs": materializing_refs,
                "owned_refs": materializing_refs,
            })
        })
        .collect::<Vec<_>>();
    let policies = user_metadata
        .extensions
        .get(POLICY_EXTENSION)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let discovery_overrides = user_metadata
        .extensions
        .get(DISCOVERY_EXTENSION)
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let structural_state = user_metadata
        .extensions
        .get(STRUCTURAL_STATE_EXTENSION)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"kind": "clean"}));
    let symbolic_targets = metadata
        .target
        .starts_with("refs/")
        .then(|| vec![metadata.target.clone()])
        .unwrap_or_default();
    let mut extensions = serde_json::Map::new();
    extensions.insert(
        "git-staircase.core".into(),
        serde_json::json!({
            "landing_policy": metadata.landing_policy,
            "verification_policy": metadata.verification_policy,
        }),
    );
    for key in [GERRIT_EXTENSION, GITHUB_EXTENSION] {
        if let Some(provider_state) = user_metadata.extensions.get(key) {
            extensions.insert(key.into(), provider_state.clone());
        }
    }
    let value = serde_json::json!({
        "schema": "git-staircase/structure",
        "version": 1,
        "kind": "linear",
        "object_format": object_format,
        "lineage_id": metadata.id,
        "integration_context": {
            "kind": "single-anchor",
            "anchors": [typed_oid(target_oid)],
            "symbolic_targets": symbolic_targets,
        },
        "steps": steps,
        "structural_state": structural_state,
        "layout": {
            "kind": metadata.primary_branch_layout.as_deref().unwrap_or("none"),
            "base": metadata.branch_layout_base,
        },
        "policies": policies,
        "discovery_overrides": discovery_overrides,
        "extensions": extensions,
        "parent_structure_revision_oid": null,
    });
    Ok(format!("{}\n", canonical_json(&value)?))
}

pub fn read_metadata(repo: &GitRepo, id_or_name: &str) -> Result<StaircaseMetadata> {
    let name_ref = StaircaseRefs::public(id_or_name);
    let id_record_ref = StaircaseRefs::state_record(id_or_name);
    let archive_ref = StaircaseRefs::archive_record(id_or_name);

    let (ref_name, is_name) = if repo.resolve_ref_opt(&name_ref)?.is_some() {
        (name_ref, true)
    } else if repo.resolve_ref_opt(&id_record_ref)?.is_some() {
        (id_record_ref, false)
    } else if repo.resolve_ref_opt(&archive_ref)?.is_some() {
        (archive_ref, false)
    } else {
        return Err(StaircaseError::Other(format!(
            "Staircase not found: {}",
            id_or_name
        )));
    };

    let record = read_record(repo, &ref_name)?;
    let mut meta = record.metadata;

    if is_name {
        meta.name = id_or_name.to_string();
    } else if meta.name.is_empty() {
        let oid = repo.resolve_ref(&ref_name)?;
        if let Ok(stdout) = repo.run(&["for-each-ref", "--points-at", &oid, PUBLIC_PREFIX]) {
            if let Some(line) = stdout.lines().next() {
                let refname = line.split_whitespace().last().unwrap_or("");
                if let Some(name) = refname.strip_prefix(PUBLIC_PREFIX) {
                    meta.name = name.to_string();
                }
            }
        }
        if meta.name.is_empty() {
            meta.name = meta.id.clone();
        }
    }
    Ok(meta)
}

pub fn parse_descriptor(content: &str) -> Result<StaircaseMetadata> {
    parse_structure(content).map(|(metadata, _)| metadata)
}

fn parse_structure(
    content: &str,
) -> Result<(StaircaseMetadata, BTreeMap<String, serde_json::Value>)> {
    let value: serde_json::Value = serde_json::from_str(content.trim_end())?;
    if value.get("schema").and_then(|value| value.as_str()) != Some("git-staircase/structure")
        || value.get("version").and_then(|value| value.as_u64()) != Some(1)
    {
        return Err(StaircaseError::Other(
            "unsupported structure schema; expected git-staircase/structure version 1".into(),
        ));
    }
    if value.get("kind").and_then(|value| value.as_str()) != Some("linear") {
        return Err(StaircaseError::UnsupportedTopology {
            operation: "read-structure".into(),
            reason: "only linear generation-1 structures are supported".into(),
        });
    }
    let object_format = required_json_str(&value, "object_format")?;
    let id = required_json_str(&value, "lineage_id")?.to_string();
    let context = value
        .get("integration_context")
        .ok_or_else(|| StaircaseError::Other("structure missing integration_context".into()))?;
    let target = context
        .get("symbolic_targets")
        .and_then(|value| value.as_array())
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            context
                .get("anchors")
                .and_then(|value| value.as_array())
                .and_then(|values| values.first())
                .and_then(|value| value.get("hex"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| StaircaseError::Other("structure has no integration anchor".into()))?;
    let integration_anchor = context
        .get("anchors")
        .and_then(|value| value.as_array())
        .and_then(|values| values.first())
        .and_then(|value| value.get("hex"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| StaircaseError::Other("structure has no integration anchor".into()))?
        .to_string();
    let step_values = value
        .get("steps")
        .and_then(|value| value.as_array())
        .ok_or_else(|| StaircaseError::Other("structure steps must be an array".into()))?;
    let mut steps = Vec::new();
    for step in step_values {
        let id = required_json_str(step, "id")?.to_string();
        let name = required_json_str(step, "name")?.to_string();
        let cut_oid = step
            .get("cut_oid")
            .ok_or_else(|| StaircaseError::Other("step missing cut_oid".into()))?;
        if required_json_str(cut_oid, "algorithm")? != object_format {
            return Err(StaircaseError::Other(
                "step cut object format does not match structure".into(),
            ));
        }
        let cut = required_json_str(cut_oid, "hex")?.to_string();
        let branch = step
            .get("materializing_refs")
            .and_then(|value| value.as_array())
            .and_then(|values| values.first())
            .and_then(|value| value.as_str())
            .map(|reference| {
                reference
                    .strip_prefix("refs/heads/")
                    .unwrap_or(reference)
                    .to_string()
            });
        if id.is_empty() || name.is_empty() || cut.is_empty() {
            return Err(StaircaseError::Other(
                "structure contains an empty or incomplete step".into(),
            ));
        }
        steps.push(Step {
            id,
            name,
            cut,
            branch,
        });
    }
    if steps.is_empty() {
        return Err(StaircaseError::Other(
            "structure must contain at least one step".into(),
        ));
    }
    let layout = value.get("layout");
    let layout_kind = layout
        .and_then(|layout| layout.get("kind"))
        .and_then(|value| value.as_str())
        .unwrap_or("none");
    let primary_branch_layout = (layout_kind != "none").then(|| layout_kind.to_string());
    let branch_layout_base = layout
        .and_then(|layout| layout.get("base"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let core_extensions = value
        .get("extensions")
        .and_then(|value| value.get("git-staircase.core"));
    let landing_policy = core_extensions
        .and_then(|value| value.get("landing_policy"))
        .cloned()
        .filter(|value| !value.is_null())
        .map(serde_json::from_value)
        .transpose()?;
    let verification_policy = core_extensions
        .and_then(|value| value.get("verification_policy"))
        .cloned()
        .filter(|value| !value.is_null())
        .map(serde_json::from_value)
        .transpose()?;
    let mut structural_extensions = BTreeMap::new();
    structural_extensions.insert(
        POLICY_EXTENSION.into(),
        value
            .get("policies")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
    );
    structural_extensions.insert(
        DISCOVERY_EXTENSION.into(),
        value
            .get("discovery_overrides")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    );
    structural_extensions.insert(
        ANCHOR_EXTENSION.into(),
        serde_json::Value::String(integration_anchor),
    );
    structural_extensions.insert(
        STRUCTURAL_STATE_EXTENSION.into(),
        value
            .get("structural_state")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({"kind": "clean"})),
    );
    if let Some(extensions) = value.get("extensions").and_then(|value| value.as_object()) {
        for key in [GERRIT_EXTENSION, GITHUB_EXTENSION] {
            if let Some(provider_state) = extensions.get(key) {
                structural_extensions.insert(key.into(), provider_state.clone());
            }
        }
    }
    Ok((
        StaircaseMetadata {
            landing_policy,
            id,
            name: String::new(),
            target,
            steps,
            verification_policy,
            primary_branch_layout,
            branch_layout_base,
            user_metadata: None,
            lifecycle: None,
        },
        structural_extensions,
    ))
}

fn required_json_str<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| StaircaseError::Other(format!("structure field '{}' must be a string", key)))
}

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

    commit_json_data(repo, &ref_name, &results, "verification.json", &commit_msg)
}

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
        let stdout = repo.run(&["for-each-ref", "--format=%(refname)", PUBLIC_PREFIX])?;
        for line in stdout.lines() {
            let refname = line.trim();
            let name = refname.strip_prefix(PUBLIC_PREFIX).unwrap_or_default();
            if name.starts_with("by-revision/") || name.ends_with("/verification") {
                continue;
            }
            let mut record = read_record(repo, refname)?;
            record.metadata.name = name.to_string();
            seen_ids.insert(record.metadata.id.clone());
            staircases.push(record.metadata);
        }

        let stdout = repo.run(&["for-each-ref", "--format=%(refname)", STATE_PREFIX])?;
        for line in stdout.lines() {
            let refname = line.trim();
            if refname.ends_with("/record") {
                let parts: Vec<&str> = refname
                    .strip_prefix(STATE_PREFIX)
                    .unwrap()
                    .split('/')
                    .collect();
                if parts.len() == 2 && !seen_ids.contains(parts[0]) {
                    let meta = read_metadata(repo, parts[0])?;
                    seen_ids.insert(meta.id.clone());
                    staircases.push(meta);
                }
            }
        }
    }

    if include_archived {
        let stdout = repo.run(&["for-each-ref", "--format=%(refname)", ARCHIVE_PREFIX])?;
        for line in stdout.lines() {
            let refname = line.trim();
            if refname.ends_with("/record") {
                let parts: Vec<&str> = refname
                    .strip_prefix(ARCHIVE_PREFIX)
                    .unwrap()
                    .split('/')
                    .collect();
                if parts.len() == 2 && parts[1] == "record" {
                    let id = parts[0];
                    if !seen_ids.contains(id) {
                        let record = read_record(repo, refname)?;
                        seen_ids.insert(record.metadata.id.clone());
                        staircases.push(record.metadata);
                    }
                }
            }
        }
    }

    Ok(staircases)
}

pub fn delete_staircase_refs(repo: &GitRepo, id: &str, name: &str) -> Result<()> {
    let mut plan = crate::core::operation::MutationPlan::new("delete", Some(id.into()));
    for prefix in [
        format!("{}{}/", STATE_PREFIX, id),
        format!("{}{}/", ARCHIVE_PREFIX, id),
    ] {
        let stdout = repo.run(&["for-each-ref", "--format=%(refname) %(objectname)", &prefix])?;
        for line in stdout.lines() {
            if let Some((reference, oid)) = line.split_once(' ') {
                plan.update(reference, Some(oid.into()), None);
            }
        }
    }
    let ref_name = StaircaseRefs::public(name);
    if let Some(oid) = repo.resolve_ref_opt(&ref_name)? {
        plan.update(ref_name, Some(oid), None);
    }
    plan.publish(repo, false).map(|_| ())
}

fn commit_json_data<T: serde::Serialize>(
    repo: &GitRepo,
    ref_name: &str,
    data: &T,
    filename: &str,
    commit_msg: &str,
) -> Result<String> {
    let blob_oid = repo.write_json(data)?;
    let entries = [TreeEntry::blob(&blob_oid, filename)];
    let tree_oid = repo.write_tree(&entries)?;

    let mut commit_args = vec!["commit-tree", &tree_oid, "-m", commit_msg];

    let parent_oid = repo.resolve_commit_opt(ref_name).unwrap_or(None);
    if let Some(ref parent) = parent_oid {
        commit_args.push("-p");
        commit_args.push(parent);
    }

    let commit_oid = repo.run(&commit_args)?;
    let commit_oid = commit_oid.trim();

    repo.run(&["update-ref", ref_name, commit_oid])?;

    Ok(commit_oid.to_string())
}

pub fn read_metadata_from_oid(repo: &GitRepo, oid: &str) -> Result<StaircaseMetadata> {
    let record = read_record(repo, oid)?;
    let mut meta = record.metadata;

    if let Ok(stdout) = repo.run(&["for-each-ref", "--points-at", oid, PUBLIC_PREFIX]) {
        if let Some(name) = stdout
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().last()?.strip_prefix(PUBLIC_PREFIX))
        {
            meta.name = name.to_string();
        }
    }
    if meta.name.is_empty() {
        meta.name = meta.id.clone();
    }
    Ok(meta)
}

pub fn read_verification(
    repo: &GitRepo,
    key: &str,
    kind: IdentityKind,
) -> Result<Option<Vec<VerificationResult>>> {
    let ref_name = match kind {
        IdentityKind::Lineage => StaircaseRefs::verification(key),
        IdentityKind::Revision => StaircaseRefs::revision_verification(&key.replace(":", "/")),
        _ => return Ok(None),
    };

    if repo.resolve_ref_opt(&ref_name)?.is_none() {
        return Ok(None);
    }

    let content = repo.run(&["cat-file", "-p", &format!("{}:verification.json", ref_name)])?;
    let results: Vec<VerificationResult> = serde_json::from_str(&content)?;
    Ok(Some(results))
}

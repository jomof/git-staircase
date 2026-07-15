use super::structure::{
    ANCHOR_EXTENSION, DISCOVERY_EXTENSION, GERRIT_EXTENSION, GITHUB_EXTENSION, POLICY_EXTENSION,
    STRUCTURAL_STATE_EXTENSION, parse_structure, serialize_structure,
};
use crate::core::refs::StaircaseRefs;
use crate::error::{Result, StaircaseError};
use crate::git::{GitRepo, TreeEntry};
use crate::model::{
    ArchiveManifest, ImplicitArchiveLifecycle, ImplicitArchiveSnapshot, ImplicitSnapshotDescriptor,
    LifecycleState, StaircaseLifecycle, StaircaseMetadata, StaircaseRecord, StaircaseUserMetadata,
};

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
        super::write_versioned_json(repo, "git-staircase-metadata 1", &descriptive_metadata)?;
    let lifecycle_oid = super::write_versioned_json(repo, "git-staircase-lifecycle 1", lifecycle)?;

    let mut entries = vec![
        TreeEntry::blob(&lifecycle_oid, "lifecycle"),
        TreeEntry::blob(&metadata_oid, "metadata"),
        TreeEntry::blob(&structure_oid, "structure"),
    ];

    let manifest_oid = if let Some(manifest) = archive_manifest {
        let m_oid =
            super::write_versioned_json(repo, "git-staircase-archive-manifest 1", manifest)?;
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

fn step_key(step: &crate::model::Step) -> &str {
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
        let old_ref = StaircaseRefs::record(&old.metadata.id, old.lifecycle.state);
        assert_ref(repo, &old_ref, expected)?;
        if old.lifecycle.state == LifecycleState::Active && !old.metadata.name.is_empty() {
            assert_ref(repo, &StaircaseRefs::public(&old.metadata.name), expected)?;
        }
    }

    let new_record_ref = StaircaseRefs::record(&metadata.id, lifecycle.state);

    match old_record.as_ref() {
        None => commands.push(format!("create {} {}", new_record_ref, new_record_oid)),
        Some(old) if old.lifecycle.state == lifecycle.state => commands.push(format!(
            "update {} {} {}",
            new_record_ref, new_record_oid, old.record_oid
        )),
        Some(old) => {
            let old_ref = StaircaseRefs::record(&old.metadata.id, old.lifecycle.state);
            commands.push(format!("delete {} {}", old_ref, old.record_oid));
            commands.push(format!("create {} {}", new_record_ref, new_record_oid));
        }
    }

    let old_public = old_record.as_ref().and_then(|record| {
        StaircaseRefs::public_optional(Some(&record.metadata.name), record.lifecycle.state)
    });
    let new_public = StaircaseRefs::public_optional(Some(&metadata.name), lifecycle.state);
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
        let old_ref = StaircaseRefs::step(
            &metadata.id,
            key,
            if old_archived {
                LifecycleState::Archived
            } else {
                LifecycleState::Active
            },
        );
        if old_archived != new_archived || !new_steps.contains_key(key) {
            if let Some(actual) = repo.resolve_ref_opt(&old_ref)? {
                commands.push(format!("delete {} {}", old_ref, actual));
            }
        }
    }
    for (key, oid) in &new_steps {
        let new_ref = StaircaseRefs::step(
            &metadata.id,
            key,
            if new_archived {
                LifecycleState::Archived
            } else {
                LifecycleState::Active
            },
        );
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
    let obj_type = repo.get_object_type(&target_oid)?;
    if obj_type != "tree" {
        return Err(StaircaseError::Other(format!(
            "record {} must be a tree, found {}",
            target_oid, obj_type
        )));
    }

    let mut entries = std::collections::BTreeMap::new();
    for entry in repo.ls_tree(&target_oid)? {
        if entry.mode != "100644" || entry.kind != "blob" {
            return Err(StaircaseError::Other(format!(
                "record entry {} must be a regular blob",
                entry.name
            )));
        }
        if !matches!(
            entry.name.as_str(),
            "structure" | "metadata" | "lifecycle" | "archive-manifest"
        ) {
            return Err(StaircaseError::Other(format!(
                "record tree {} contains unknown entry {}",
                target_oid, entry.name
            )));
        }
        entries.insert(entry.name, entry.oid);
    }

    let structure_oid = required_entry(&entries, "structure", &target_oid)?;
    let metadata_oid = required_entry(&entries, "metadata", &target_oid)?;
    let lifecycle_oid = required_entry(&entries, "lifecycle", &target_oid)?;
    let manifest_oid = entries.get("archive-manifest").cloned();

    let structure = repo.cat_file(&structure_oid)?;
    let (mut metadata, structural_extensions) = parse_structure(&structure)?;
    let mut user_metadata: StaircaseUserMetadata =
        super::read_versioned_json(repo, &metadata_oid, "git-staircase-metadata 1")?;
    user_metadata.extensions.extend(structural_extensions);
    let lifecycle: StaircaseLifecycle =
        super::read_versioned_json(repo, &lifecycle_oid, "git-staircase-lifecycle 1")?;
    let archive_manifest: Option<ArchiveManifest> = manifest_oid
        .as_ref()
        .map(|oid| super::read_versioned_json(repo, oid, "git-staircase-archive-manifest 1"))
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

pub fn write_metadata(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    let mut metadata = metadata.clone();
    for step in &mut metadata.steps {
        if step.id.is_empty() {
            step.id = uuid::Uuid::new_v4().to_string();
        }
    }
    let user_metadata = metadata.user_metadata.clone().unwrap_or_default();
    let lifecycle = metadata.lifecycle.clone().unwrap_or_default();
    let current_ref = StaircaseRefs::record(&metadata.id, lifecycle.state);
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
        if let Ok(lines) =
            repo.for_each_ref(crate::core::refs::PUBLIC_PREFIX, "%(refname)", Some(&oid))
        {
            if let Some(refname) = lines.first() {
                if let Some(name) = refname.strip_prefix(crate::core::refs::PUBLIC_PREFIX) {
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

pub fn read_metadata_from_oid(repo: &GitRepo, oid: &str) -> Result<StaircaseMetadata> {
    let record = read_record(repo, oid)?;
    let mut meta = record.metadata;
    if meta.name.is_empty() {
        if let Ok(lines) =
            repo.for_each_ref(crate::core::refs::PUBLIC_PREFIX, "%(refname)", Some(oid))
        {
            if let Some(refname) = lines.first() {
                if let Some(name) = refname.strip_prefix(crate::core::refs::PUBLIC_PREFIX) {
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

pub fn write_implicit_archive_snapshot(
    repo: &GitRepo,
    descriptor: &ImplicitSnapshotDescriptor,
    lifecycle: &ImplicitArchiveLifecycle,
    manifest: &ArchiveManifest,
) -> Result<ImplicitArchiveSnapshot> {
    let kind_oid = repo.write_blob("implicit-snapshot\n")?;
    let descriptor_oid =
        super::write_versioned_json(repo, "git-staircase-implicit-snapshot 1", descriptor)?;
    let lifecycle_oid =
        super::write_versioned_json(repo, "git-staircase-implicit-lifecycle 1", lifecycle)?;
    let manifest_oid =
        super::write_versioned_json(repo, "git-staircase-archive-manifest 1", manifest)?;

    let entries = vec![
        TreeEntry::blob(&manifest_oid, "archive-manifest"),
        TreeEntry::blob(&kind_oid, "kind"),
        TreeEntry::blob(&lifecycle_oid, "lifecycle"),
        TreeEntry::blob(&descriptor_oid, "snapshot"),
    ];

    let record_oid = repo.write_tree(&entries)?;
    let record_ref = StaircaseRefs::implicit_archive_record(&descriptor.archive_id);

    let mut commands = vec![format!("create {} {}", record_ref, record_oid)];

    for (idx, cut_oid) in descriptor.ordered_cuts.iter().enumerate() {
        let cut_ref = StaircaseRefs::implicit_archive_cut(&descriptor.archive_id, idx + 1);
        commands.push(format!("create {} {}", cut_ref, cut_oid));
    }

    for owned in &manifest.owned_refs {
        let owned_ref =
            StaircaseRefs::implicit_archive_owned(&descriptor.archive_id, &owned.ref_id);
        commands.push(format!("create {} {}", owned_ref, owned.original_oid));
    }

    let mut plan = crate::core::operation::MutationPlan::new(
        "implicit-archive-record",
        Some(descriptor.archive_id.clone()),
    );
    for cmd in &commands {
        let fields: Vec<_> = cmd.split_whitespace().collect();
        if let ["create", r, new] = fields.as_slice() {
            plan.update((*r).to_string(), None, Some((*new).to_string()));
        }
    }
    plan.publish(repo, false)?;

    let mut metadata = StaircaseMetadata {
        landing_policy: None,
        id: descriptor.archive_id.clone(),
        verification_policy: None,
        name: descriptor.canonical_display_name.clone(),
        symbolic_integration_target: descriptor.integration_context.clone(),
        steps: descriptor
            .ordered_cuts
            .iter()
            .enumerate()
            .map(|(i, cut)| crate::model::Step {
                id: String::new(),
                name: format!("step-{}", i + 1),
                cut: cut.clone(),
                branch: None,
            })
            .collect(),
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: Some(StaircaseLifecycle {
            state: LifecycleState::Archived,
            archive_reason: lifecycle.reason.clone(),
            name_reserved: lifecycle.name_reservation,
            events: vec![],
        }),
    };
    metadata.user_metadata = Some(StaircaseUserMetadata::default());

    Ok(ImplicitArchiveSnapshot {
        archive_id: descriptor.archive_id.clone(),
        record_oid,
        descriptor: descriptor.clone(),
        lifecycle: lifecycle.clone(),
        manifest: manifest.clone(),
        metadata,
    })
}

pub fn read_implicit_archive_snapshot(
    repo: &GitRepo,
    target: &str,
) -> Result<ImplicitArchiveSnapshot> {
    let target_ref = if target.starts_with("refs/") {
        target.to_string()
    } else if target.starts_with("archive@") {
        let aid = target.strip_prefix("archive@").unwrap();
        StaircaseRefs::implicit_archive_record(aid)
    } else {
        StaircaseRefs::implicit_archive_record(target)
    };

    let target_oid = repo
        .resolve_ref_opt(&target_ref)?
        .unwrap_or_else(|| target.to_string());

    let obj_type = repo.get_object_type(&target_oid)?;
    if obj_type != "tree" {
        return Err(StaircaseError::Other(format!(
            "implicit archive record {} must be a tree, found {}",
            target_oid, obj_type
        )));
    }

    let mut entries = std::collections::BTreeMap::new();
    for entry in repo.ls_tree(&target_oid)? {
        entries.insert(entry.name, entry.oid);
    }

    let descriptor_oid = required_entry(&entries, "snapshot", &target_oid)?;
    let lifecycle_oid = required_entry(&entries, "lifecycle", &target_oid)?;
    let manifest_oid = required_entry(&entries, "archive-manifest", &target_oid)?;

    let descriptor: ImplicitSnapshotDescriptor =
        super::read_versioned_json(repo, &descriptor_oid, "git-staircase-implicit-snapshot 1")?;
    let lifecycle: ImplicitArchiveLifecycle =
        super::read_versioned_json(repo, &lifecycle_oid, "git-staircase-implicit-lifecycle 1")?;
    let manifest: ArchiveManifest =
        super::read_versioned_json(repo, &manifest_oid, "git-staircase-archive-manifest 1")?;

    let mut metadata = StaircaseMetadata {
        landing_policy: None,
        id: descriptor.archive_id.clone(),
        verification_policy: None,
        name: descriptor.canonical_display_name.clone(),
        symbolic_integration_target: descriptor.integration_context.clone(),
        steps: descriptor
            .ordered_cuts
            .iter()
            .enumerate()
            .map(|(i, cut)| crate::model::Step {
                id: String::new(),
                name: format!("step-{}", i + 1),
                cut: cut.clone(),
                branch: None,
            })
            .collect(),
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: Some(StaircaseLifecycle {
            state: LifecycleState::Archived,
            archive_reason: lifecycle.reason.clone(),
            name_reserved: lifecycle.name_reservation,
            events: vec![],
        }),
    };
    metadata.user_metadata = Some(StaircaseUserMetadata::default());

    Ok(ImplicitArchiveSnapshot {
        archive_id: descriptor.archive_id.clone(),
        record_oid: target_oid,
        descriptor,
        lifecycle,
        manifest,
        metadata,
    })
}

pub fn list_implicit_archive_snapshots(repo: &GitRepo) -> Result<Vec<ImplicitArchiveSnapshot>> {
    let mut snapshots = Vec::new();
    if let Ok(lines) = repo.for_each_ref(StaircaseRefs::IMPLICIT_ARCHIVE_PREFIX, "%(refname)", None)
    {
        for line in lines {
            let refname = line.trim();
            if refname.ends_with("/record") {
                if let Ok(snap) = read_implicit_archive_snapshot(repo, refname) {
                    snapshots.push(snap);
                }
            }
        }
    }
    Ok(snapshots)
}

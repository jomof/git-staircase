use crate::core::refs::{ARCHIVE_PREFIX, PUBLIC_PREFIX, STATE_PREFIX, StaircaseRefs};
use crate::error::{Result, StaircaseError};
use crate::git::{GitRepo, TreeEntry};
use crate::model::{
    ArchiveManifest, IdentityKind, LifecycleState, StaircaseLifecycle, StaircaseMetadata,
    StaircaseRecord, StaircaseUserMetadata, Step, VerificationPolicy, VerificationResult,
};

pub fn write_record(
    repo: &GitRepo,
    metadata: &StaircaseMetadata,
    user_metadata: &StaircaseUserMetadata,
    lifecycle: &StaircaseLifecycle,
    archive_manifest: Option<&ArchiveManifest>,
    update_refs: bool,
) -> Result<StaircaseRecord> {
    let structure_desc = serialize_descriptor(repo, metadata)?;
    let structure_oid = repo.write_blob(&structure_desc)?;

    let metadata_oid = repo.write_json(user_metadata)?;
    let lifecycle_oid = repo.write_json(lifecycle)?;

    let mut entries = vec![
        TreeEntry::blob(&structure_oid, "structure"),
        TreeEntry::blob(&metadata_oid, "metadata"),
        TreeEntry::blob(&lifecycle_oid, "lifecycle"),
    ];

    let manifest_oid = if let Some(manifest) = archive_manifest {
        let m_oid = repo.write_json(manifest)?;
        entries.push(TreeEntry::blob(&m_oid, "archive-manifest"));
        Some(m_oid)
    } else {
        None
    };

    let record_oid = repo.write_tree(&entries)?;

    let mut full_meta = metadata.clone();
    full_meta.user_metadata = Some(user_metadata.clone());
    full_meta.lifecycle = Some(lifecycle.clone());

    if update_refs {
        if lifecycle.state == LifecycleState::Active {
            if !metadata.name.is_empty() {
                let public_ref = StaircaseRefs::public(&metadata.name);
                repo.run(&["update-ref", &public_ref, &record_oid])?;
            }
            let state_record_ref = StaircaseRefs::state_record(&metadata.id);
            repo.run(&["update-ref", &state_record_ref, &record_oid])?;

            let state_desc_ref = StaircaseRefs::state_descriptor(&metadata.id);
            repo.run(&["update-ref", &state_desc_ref, &structure_oid])?;

            for step in &metadata.steps {
                let key = if !step.id.is_empty() {
                    &step.id
                } else {
                    &step.name
                };
                let step_ref = StaircaseRefs::state_step(&metadata.id, key);
                repo.run(&["update-ref", &step_ref, &step.cut])?;
            }
        } else if lifecycle.state == LifecycleState::Archived {
            let archive_record_ref = StaircaseRefs::archive_record(&metadata.id);
            repo.run(&["update-ref", &archive_record_ref, &record_oid])?;

            for step in &metadata.steps {
                let key = if !step.id.is_empty() {
                    &step.id
                } else {
                    &step.name
                };
                let step_ref = StaircaseRefs::archive_step(&metadata.id, key);
                repo.run(&["update-ref", &step_ref, &step.cut])?;
            }
        }
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

pub fn read_record(repo: &GitRepo, target: &str) -> Result<StaircaseRecord> {
    let mut target_oid = repo
        .resolve_ref_opt(target)?
        .unwrap_or_else(|| target.to_string());

    let mut obj_type = repo
        .run(&["cat-file", "-t", &target_oid])
        .unwrap_or_default()
        .trim()
        .to_string();

    if obj_type == "commit" {
        if let Ok(tree_oid) = repo.run(&["rev-parse", &format!("{}^{{tree}}", target_oid)]) {
            target_oid = tree_oid.trim().to_string();
            obj_type = "tree".to_string();
        }
    }

    if obj_type == "blob" {
        let content = repo.run(&["cat-file", "-p", &target_oid])?;
        let mut meta = parse_descriptor(&content)?;
        let user_meta = meta.user_metadata.clone().unwrap_or_default();
        let lifecycle = meta.lifecycle.clone().unwrap_or_default();

        let metadata_oid = repo.write_json(&user_meta)?;
        let lifecycle_oid = repo.write_json(&lifecycle)?;

        meta.user_metadata = Some(user_meta.clone());
        meta.lifecycle = Some(lifecycle.clone());

        return Ok(StaircaseRecord {
            record_oid: target_oid.clone(),
            structure_oid: target_oid,
            metadata_oid,
            lifecycle_oid,
            archive_manifest_oid: None,
            metadata: meta,
            user_metadata: user_meta,
            lifecycle,
            archive_manifest: None,
        });
    }

    if obj_type == "tree" {
        let ls_output = repo.run(&["ls-tree", &target_oid])?;
        let mut structure_oid = None;
        let mut metadata_oid = None;
        let mut lifecycle_oid = None;
        let mut manifest_oid = None;

        for line in ls_output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let oid = parts[2].to_string();
                let name = parts[3];
                match name {
                    "structure" => structure_oid = Some(oid),
                    "metadata" => metadata_oid = Some(oid),
                    "lifecycle" => lifecycle_oid = Some(oid),
                    "archive-manifest" => manifest_oid = Some(oid),
                    _ => {}
                }
            }
        }

        let struct_oid = structure_oid.ok_or_else(|| {
            StaircaseError::Other(format!(
                "Record tree {} missing 'structure' entry",
                target_oid
            ))
        })?;

        let struct_content = repo.run(&["cat-file", "-p", &struct_oid])?;
        let mut metadata = parse_descriptor(&struct_content)?;

        let user_metadata: StaircaseUserMetadata = if let Some(ref m_oid) = metadata_oid {
            let m_content = repo.run(&["cat-file", "-p", m_oid])?;
            serde_json::from_str(&m_content).unwrap_or_default()
        } else {
            StaircaseUserMetadata::default()
        };

        let lifecycle: StaircaseLifecycle = if let Some(ref l_oid) = lifecycle_oid {
            let l_content = repo.run(&["cat-file", "-p", l_oid])?;
            serde_json::from_str(&l_content).unwrap_or_default()
        } else {
            StaircaseLifecycle::default()
        };

        let archive_manifest: Option<ArchiveManifest> = if let Some(ref am_oid) = manifest_oid {
            let am_content = repo.run(&["cat-file", "-p", am_oid])?;
            serde_json::from_str(&am_content).ok()
        } else {
            None
        };

        if metadata.name.is_empty() {
            if let Some(ref manifest) = archive_manifest {
                metadata.name = manifest.canonical_name.clone();
            }
        }

        metadata.user_metadata = Some(user_metadata.clone());
        metadata.lifecycle = Some(lifecycle.clone());

        return Ok(StaircaseRecord {
            record_oid: target_oid,
            structure_oid: struct_oid,
            metadata_oid: metadata_oid.unwrap_or_default(),
            lifecycle_oid: lifecycle_oid.unwrap_or_default(),
            archive_manifest_oid: manifest_oid,
            metadata,
            user_metadata,
            lifecycle,
            archive_manifest,
        });
    }

    Err(StaircaseError::Other(format!(
        "Target {} is not a blob or tree (type: {})",
        target, obj_type
    )))
}

pub fn write_metadata(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    let user_metadata = metadata.user_metadata.clone().unwrap_or_default();
    let lifecycle = metadata.lifecycle.clone().unwrap_or_default();
    let record = write_record(repo, metadata, &user_metadata, &lifecycle, None, true)?;
    Ok(record.record_oid)
}

pub fn serialize_descriptor(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    let mut out = String::new();
    out.push_str("git-staircase-descriptor 1\n");

    if let Ok(format) = repo.get_object_format() {
        out.push_str(&format!("object-format {}\n", format));
    }

    out.push_str(&format!("lineage {}\n", metadata.id));
    out.push_str(&format!("target-ref {}\n", metadata.target));

    if let Some(ref policy) = metadata.verification_policy {
        if let Some(ref build) = policy.build_command {
            out.push_str(&format!("build-command {}\n", build));
        }
        if let Some(ref test) = policy.test_command {
            out.push_str(&format!("test-command {}\n", test));
        }
        out.push_str(&format!(
            "verify-each-prefix {}\n",
            policy.verify_each_prefix
        ));
    }

    for step in &metadata.steps {
        out.push_str("\n");
        out.push_str(&format!("step {}\n", step.name));
        out.push_str(&format!("step-id {}\n", step.id));
        out.push_str(&format!("cut {}\n", step.cut));
        if let Some(ref branch) = step.branch {
            let full_ref = if branch.starts_with("refs/") {
                branch.clone()
            } else {
                format!("refs/heads/{}", branch)
            };
            out.push_str(&format!("materializing-ref {}\n", full_ref));
        }
    }

    Ok(out)
}

pub fn read_metadata(repo: &GitRepo, id_or_name: &str) -> Result<StaircaseMetadata> {
    let name_ref = StaircaseRefs::public(id_or_name);
    let id_record_ref = StaircaseRefs::state_record(id_or_name);
    let id_desc_ref = StaircaseRefs::state_descriptor(id_or_name);
    let archive_ref = StaircaseRefs::archive_record(id_or_name);

    let (ref_name, is_name) = if repo.resolve_ref_opt(&name_ref)?.is_some() {
        (name_ref, true)
    } else if repo.resolve_ref_opt(&id_record_ref)?.is_some() {
        (id_record_ref, false)
    } else if repo.resolve_ref_opt(&id_desc_ref)?.is_some() {
        (id_desc_ref, false)
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
    if content.starts_with("git-staircase-descriptor 1\n") {
        return parse_canonical_descriptor(content);
    }

    if let Ok(meta) = serde_json::from_str::<StaircaseMetadata>(content) {
        return Ok(meta);
    }

    parse_legacy_descriptor(content)
}

fn parse_canonical_descriptor(content: &str) -> Result<StaircaseMetadata> {
    let mut id = String::new();
    let mut target = String::new();
    let mut steps = Vec::new();
    let mut current_step: Option<Step> = None;
    let mut verification_policy = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line == "git-staircase-descriptor 1" {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }

        match parts[0] {
            "step-id" => {
                if let Some(ref mut step) = current_step {
                    step.id = parts[1].to_string();
                }
            }
            "lineage" => id = parts[1].to_string(),
            "target-ref" => target = parts[1].to_string(),
            "build-command" => {
                let policy = verification_policy.get_or_insert(VerificationPolicy {
                    build_command: None,
                    test_command: None,
                    verify_each_prefix: false,
                });
                policy.build_command = Some(parts[1].to_string());
            }
            "test-command" => {
                let policy = verification_policy.get_or_insert(VerificationPolicy {
                    build_command: None,
                    test_command: None,
                    verify_each_prefix: false,
                });
                policy.test_command = Some(parts[1].to_string());
            }
            "verify-each-prefix" => {
                let policy = verification_policy.get_or_insert(VerificationPolicy {
                    build_command: None,
                    test_command: None,
                    verify_each_prefix: false,
                });
                policy.verify_each_prefix = parts[1] == "true";
            }
            "step" => {
                if let Some(step) = current_step.take() {
                    steps.push(step);
                }
                current_step = Some(Step {
                    id: String::new(),
                    name: parts[1].to_string(),
                    cut: String::new(),
                    branch: None,
                });
            }
            "cut" => {
                if let Some(ref mut step) = current_step {
                    step.cut = parts[1].to_string();
                }
            }
            "materializing-ref" => {
                if let Some(ref mut step) = current_step {
                    let b = parts[1].strip_prefix("refs/heads/").unwrap_or(parts[1]);
                    step.branch = Some(b.to_string());
                }
            }
            _ => {}
        }
    }

    if let Some(step) = current_step {
        steps.push(step);
    }

    Ok(StaircaseMetadata {
        landing_policy: None,
        id,
        name: String::new(),
        target,
        steps,
        verification_policy,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    })
}

fn parse_legacy_descriptor(content: &str) -> Result<StaircaseMetadata> {
    parse_canonical_descriptor(content)
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
        if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", PUBLIC_PREFIX]) {
            for line in stdout.lines() {
                let refname = line.trim();
                if refname.starts_with(PUBLIC_PREFIX) {
                    let name = refname.strip_prefix(PUBLIC_PREFIX).unwrap();
                    if !name.contains('/') {
                        if let Ok(meta) = read_metadata(repo, name) {
                            seen_ids.insert(meta.id.clone());
                            staircases.push(meta);
                        }
                    }
                }
            }
        }

        if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", STATE_PREFIX]) {
            for line in stdout.lines() {
                let refname = line.trim();
                if refname.ends_with("/record") || refname.ends_with("/descriptor") {
                    let parts: Vec<&str> = refname
                        .strip_prefix(STATE_PREFIX)
                        .unwrap()
                        .split('/')
                        .collect();
                    if parts.len() == 2 {
                        let id = parts[0];
                        if !seen_ids.contains(id) {
                            if let Ok(meta) = read_metadata(repo, id) {
                                seen_ids.insert(meta.id.clone());
                                staircases.push(meta);
                            }
                        }
                    }
                }
            }
        }
    }

    if include_archived {
        if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", ARCHIVE_PREFIX]) {
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
                            if let Ok(record) = read_record(repo, refname) {
                                seen_ids.insert(record.metadata.id.clone());
                                staircases.push(record.metadata);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(staircases)
}

pub fn delete_staircase_refs(repo: &GitRepo, id: &str, name: &str) -> Result<()> {
    let state_prefix = format!("{}{}/", STATE_PREFIX, id);
    if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", &state_prefix]) {
        for line in stdout.lines() {
            let refname = line.trim();
            repo.run(&["update-ref", "-d", refname])?;
        }
    }
    let archive_prefix = format!("{}{}/", ARCHIVE_PREFIX, id);
    if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", &archive_prefix]) {
        for line in stdout.lines() {
            let refname = line.trim();
            repo.run(&["update-ref", "-d", refname])?;
        }
    }
    let ref_name = StaircaseRefs::public(name);
    repo.run(&["update-ref", "-d", &ref_name])?;
    Ok(())
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

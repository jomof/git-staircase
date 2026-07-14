use crate::core::persistence;
use crate::core::refs::StaircaseRefs;
use crate::core::resolved::ResolvedSelector;
use crate::core::utils::current_timestamp;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{LifecycleEvent, LifecycleState};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UnarchiveBranchesMode {
    #[default]
    Exact,
    Rename,
    None,
}

#[derive(Debug, Clone, Default)]
pub struct UnarchiveOptions {
    pub new_name: Option<String>,
    pub branch_base: Option<String>,
    pub branches_mode: UnarchiveBranchesMode,
    pub adopt_existing_branches: bool,
    pub reattach_worktrees: bool,
    pub adopt: bool,
    pub accept_current_context: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UnarchiveResult {
    pub restored_staircase_id: String,
    pub canonical_name: String,
    pub restored_branches: Vec<String>,
}

pub fn unarchive_staircase(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    options: &UnarchiveOptions,
) -> Result<UnarchiveResult> {
    if let crate::core::ResolvedStaircase::ImplicitArchive(ref snap) = selector.staircase {
        return unarchive_implicit_archive_snapshot(repo, snap, options);
    }

    let meta = selector.staircase.metadata();
    let current_ref = StaircaseRefs::record(
        &meta.id,
        meta.lifecycle
            .as_ref()
            .map(|l| l.state)
            .unwrap_or(LifecycleState::Archived),
    );
    let record = persistence::read_record(repo, &current_ref)?;

    if record.lifecycle.state == LifecycleState::Active {
        return Ok(UnarchiveResult {
            restored_staircase_id: meta.id.clone(),
            canonical_name: meta.name.clone(),
            restored_branches: Vec::new(),
        });
    }

    let target_name = options
        .new_name
        .clone()
        .unwrap_or_else(|| record.metadata.name.clone());

    let public_ref = StaircaseRefs::public(&target_name);
    if let Ok(existing_record_oid) = repo.resolve_ref_opt(&public_ref) {
        if let Some(oid) = existing_record_oid {
            if let Ok(existing_record) = persistence::read_record(repo, &oid) {
                if existing_record.metadata.id != meta.id {
                    return Err(StaircaseError::Other(format!(
                        "canonical staircase name '{}' collides with an active staircase; specify --name <new-name>",
                        target_name
                    )));
                }
            }
        }
    }

    let mut restored_branches = Vec::new();

    if options.branches_mode != UnarchiveBranchesMode::None {
        for (idx, step) in record.metadata.steps.iter().enumerate() {
            let dest_branch_name = if let Some(ref base) = options.branch_base {
                if record.metadata.steps.len() > 1 {
                    format!("{}-{}", base, idx + 1)
                } else {
                    base.clone()
                }
            } else if let Some(ref b) = step.branch {
                b.strip_prefix("refs/heads/").unwrap_or(b).to_string()
            } else {
                format!("{}-{}", target_name, idx + 1)
            };

            let dest_ref = format!("refs/heads/{}", dest_branch_name);

            if let Ok(Some(existing_oid)) = repo.resolve_ref_opt(&dest_ref) {
                if existing_oid == step.cut {
                    if !options.adopt_existing_branches {
                        return Err(StaircaseError::Other(format!(
                            "branch '{}' exists at step cut {}; adoption requires --adopt-existing-branches",
                            dest_ref, existing_oid
                        )));
                    }
                } else {
                    return Err(StaircaseError::Other(format!(
                        "cannot restore {}: branch exists at different OID ({}, expected {})",
                        dest_ref, existing_oid, step.cut
                    )));
                }
            }

            if let Ok(stdout) = repo.run(&[
                "config",
                "--get-regexp",
                &format!("^branch\\.{}\\.", dest_branch_name),
            ]) {
                if !stdout.trim().is_empty() {
                    let is_owned = record
                        .archive_manifest
                        .as_ref()
                        .map(|m| {
                            m.branch_configs
                                .iter()
                                .any(|bc| bc.branch_name == dest_branch_name)
                        })
                        .unwrap_or(false);
                    if !is_owned {
                        return Err(StaircaseError::Other(format!(
                            "configuration for branch '{}' exists and is not owned by the archived staircase",
                            dest_branch_name
                        )));
                    }
                }
            }

            restored_branches.push((dest_branch_name, step.cut.clone()));
        }
    }

    let event_id = format!("evt-unarchive-{}", uuid::Uuid::new_v4().simple());
    let mut updated_lifecycle = record.lifecycle.clone();
    updated_lifecycle.state = LifecycleState::Active;
    updated_lifecycle.events.push(LifecycleEvent {
        event_id: event_id.clone(),
        kind: "unarchived".to_string(),
        timestamp: current_timestamp(),
        actor: None,
        record_oid_before: Some(record.record_oid.clone()),
        record_oid_after: None,
        canonical_name: Some(target_name.clone()),
        reason: None,
        details: serde_json::Value::Null,
    });

    let mut updated_metadata = record.metadata.clone();
    updated_metadata.name = target_name.clone();

    if options.branches_mode != UnarchiveBranchesMode::None {
        for (idx, (b_name, _)) in restored_branches.iter().enumerate() {
            if idx < updated_metadata.steps.len() {
                updated_metadata.steps[idx].branch = Some(b_name.clone());
            }
        }
    } else {
        for step in &mut updated_metadata.steps {
            step.branch = None;
        }
    }

    let active_record = persistence::write_record(
        repo,
        &updated_metadata,
        &record.user_metadata,
        &updated_lifecycle,
        None,
        Some(&record.record_oid),
        false,
    )?;
    let mut plan = crate::core::operation::MutationPlan::new("unarchive", Some(meta.id.clone()))
        .expected_record(Some(record.record_oid.clone()));
    plan.update(
        StaircaseRefs::record(&meta.id, LifecycleState::Archived),
        Some(record.record_oid.clone()),
        None,
    );
    plan.update(
        StaircaseRefs::record(&meta.id, LifecycleState::Active),
        None,
        Some(active_record.record_oid.clone()),
    );
    plan.update(
        StaircaseRefs::public(&target_name),
        None,
        Some(active_record.record_oid.clone()),
    );
    for step in &updated_metadata.steps {
        let key = if step.id.is_empty() {
            &step.name
        } else {
            &step.id
        };
        plan.update(
            StaircaseRefs::step(&meta.id, key, LifecycleState::Archived),
            Some(step.cut.clone()),
            None,
        );
        plan.update(
            StaircaseRefs::step(&meta.id, key, LifecycleState::Active),
            None,
            Some(step.cut.clone()),
        );
    }
    if let Some(manifest) = &record.archive_manifest {
        for owned in &manifest.owned_refs {
            if repo.resolve_ref_opt(&owned.archive_refname)?.is_some() {
                plan.update(
                    owned.archive_refname.clone(),
                    Some(owned.original_oid.clone()),
                    None,
                );
            }
        }
    }
    for (b_name, cut_oid) in &restored_branches {
        let full_ref = format!("refs/heads/{}", b_name);
        plan.update(
            full_ref.clone(),
            repo.resolve_ref_opt(&full_ref)?,
            Some(cut_oid.clone()),
        );
    }
    plan.publish(repo, false)?;

    if let Some(ref manifest) = record.archive_manifest {
        for bc in &manifest.branch_configs {
            for entry in &bc.entries {
                let _ = repo.run(&["config", &entry.key, &entry.value]);
            }
        }
    }

    Ok(UnarchiveResult {
        restored_staircase_id: meta.id.clone(),
        canonical_name: target_name,
        restored_branches: restored_branches.into_iter().map(|(b, _)| b).collect(),
    })
}

fn unarchive_implicit_archive_snapshot(
    repo: &GitRepo,
    snap: &crate::model::ImplicitArchiveSnapshot,
    options: &UnarchiveOptions,
) -> Result<UnarchiveResult> {
    let mut restored_branches = Vec::new();

    if options.branches_mode != UnarchiveBranchesMode::None {
        for (idx, cut) in snap.descriptor.ordered_cuts.iter().enumerate() {
            let branch_name = if let Some(ref base) = options.branch_base {
                if snap.descriptor.ordered_cuts.len() > 1 {
                    format!("{}-{}", base, idx + 1)
                } else {
                    base.clone()
                }
            } else if let Some(owned) = snap.manifest.owned_refs.get(idx) {
                owned
                    .original_refname
                    .strip_prefix("refs/heads/")
                    .unwrap_or(&owned.original_refname)
                    .to_string()
            } else {
                format!("{}-{}", snap.descriptor.canonical_display_name, idx + 1)
            };

            let dest_ref = format!("refs/heads/{}", branch_name);

            if let Ok(Some(existing_oid)) = repo.resolve_ref_opt(&dest_ref) {
                if existing_oid == *cut {
                    if !options.adopt_existing_branches {
                        return Err(StaircaseError::Other(format!(
                            "branch '{}' exists at step cut {}; adoption requires --adopt-existing-branches",
                            dest_ref, existing_oid
                        )));
                    }
                } else {
                    return Err(StaircaseError::Other(format!(
                        "cannot restore {}: branch exists at different OID ({}, expected {})",
                        dest_ref, existing_oid, cut
                    )));
                }
            }

            restored_branches.push((branch_name, cut.clone()));
        }
    }

    if options.adopt {
        let mut meta = snap.metadata.clone();
        if let Some(ref new_name) = options.new_name {
            meta.name = new_name.clone();
        }
        for (branch_name, cut_oid) in &restored_branches {
            repo.run(&["branch", branch_name, cut_oid])?;
        }
        delete_implicit_archive_refs(repo, &snap.archive_id)?;

        let adopted = crate::core::resolved::adopt(repo, &meta)?;
        return Ok(UnarchiveResult {
            restored_staircase_id: adopted.id,
            canonical_name: adopted.name,
            restored_branches: restored_branches.into_iter().map(|(b, _)| b).collect(),
        });
    }

    for (branch_name, cut_oid) in &restored_branches {
        repo.run(&["branch", branch_name, cut_oid])?;
    }

    for bc in &snap.manifest.branch_configs {
        for entry in &bc.entries {
            let _ = repo.run(&["config", &entry.key, &entry.value]);
        }
    }

    delete_implicit_archive_refs(repo, &snap.archive_id)?;

    Ok(UnarchiveResult {
        restored_staircase_id: snap.archive_id.clone(),
        canonical_name: snap.descriptor.canonical_display_name.clone(),
        restored_branches: restored_branches.into_iter().map(|(b, _)| b).collect(),
    })
}

fn delete_implicit_archive_refs(repo: &GitRepo, archive_id: &str) -> Result<()> {
    let mut commands = Vec::new();
    let record_ref = StaircaseRefs::implicit_archive_record(archive_id);
    if let Some(oid) = repo.resolve_ref_opt(&record_ref)? {
        commands.push(format!("delete {} {}", record_ref, oid));
    }

    let cut_prefix = format!("{}{}/cuts/", StaircaseRefs::IMPLICIT_ARCHIVE_PREFIX, archive_id);
    if let Ok(lines) = repo.for_each_ref(&cut_prefix, "%(refname) %(objectname)", None) {
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                commands.push(format!("delete {} {}", parts[0], parts[1]));
            }
        }
    }

    let owned_prefix = format!("{}{}/owned/", StaircaseRefs::IMPLICIT_ARCHIVE_PREFIX, archive_id);
    if let Ok(lines) = repo.for_each_ref(&owned_prefix, "%(refname) %(objectname)", None) {
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                commands.push(format!("delete {} {}", parts[0], parts[1]));
            }
        }
    }

    if !commands.is_empty() {
        repo.update_refs_transaction(&commands)?;
    }
    Ok(())
}

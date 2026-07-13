use crate::core::persistence;
use crate::core::refs::{STATE_PREFIX, StaircaseRefs};
use crate::core::resolved::ResolvedSelector;
use crate::core::utils::current_timestamp;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{
    ArchiveManifest, ArchivedOwnedRef, BranchConfigEntry, BranchConfigSnapshot, LifecycleEvent,
    LifecycleState,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ArchiveOptions {
    pub reason: Option<String>,
    pub dry_run: bool,
    pub snapshot_drafts: bool,
    pub detach_dirty_worktrees: bool,
    pub leave_worktrees: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ArchiveResult {
    pub archived_staircase_id: String,
    pub canonical_name: String,
    pub archive_event_id: String,
    pub moved_branches: Vec<String>,
    pub unowned_warnings: Vec<String>,
    pub is_dry_run: bool,
}

pub fn archive_staircase(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    options: &ArchiveOptions,
) -> Result<ArchiveResult> {
    let repo_path = &repo.workdir;
    let git_dir = repo_path.join(".git");
    if git_dir.join("rebase-apply").exists()
        || git_dir.join("rebase-merge").exists()
        || git_dir.join("MERGE_HEAD").exists()
        || git_dir.join("CHERRY_PICK_HEAD").exists()
        || git_dir.join("REVERT_HEAD").exists()
    {
        return Err(StaircaseError::Other(
            "active Git operation in progress; finish or abort it before archiving".to_string(),
        ));
    }

    let meta = selector.staircase.metadata();
    let record_ref = StaircaseRefs::state_record(&meta.id);
    let archive_record_ref = StaircaseRefs::archive_record(&meta.id);

    let current_record = persistence::read_record(repo, &record_ref)
        .or_else(|_| persistence::read_record(repo, &StaircaseRefs::public(&meta.name)))
        .or_else(|_| persistence::read_record(repo, &archive_record_ref))?;

    if current_record.lifecycle.state == LifecycleState::Archived {
        return Ok(ArchiveResult {
            archived_staircase_id: meta.id.clone(),
            canonical_name: meta.name.clone(),
            archive_event_id: current_record
                .lifecycle
                .events
                .last()
                .map(|e| e.event_id.clone())
                .unwrap_or_default(),
            moved_branches: Vec::new(),
            unowned_warnings: Vec::new(),
            is_dry_run: options.dry_run,
        });
    }

    let mut owned_branches = Vec::new();
    for step in &meta.steps {
        if let Some(ref b) = step.branch {
            let full_ref = if b.starts_with("refs/") {
                b.clone()
            } else {
                format!("refs/heads/{}", b)
            };
            if !owned_branches.contains(&full_ref) {
                owned_branches.push(full_ref);
            }
        }
    }

    let mut branch_configs = Vec::new();
    for full_ref in &owned_branches {
        let branch_name = full_ref.strip_prefix("refs/heads/").unwrap_or(full_ref);
        let mut entries = Vec::new();
        if let Ok(stdout) = repo.run(&[
            "config",
            "--get-regexp",
            &format!("^branch\\.{}\\.", branch_name),
        ]) {
            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    entries.push(BranchConfigEntry {
                        key: parts[0].to_string(),
                        value: parts[1].to_string(),
                    });
                }
            }
        }
        branch_configs.push(BranchConfigSnapshot {
            branch_name: branch_name.to_string(),
            entries,
        });
    }

    if let Ok(stdout) = repo.run(&["worktree", "list", "--porcelain"]) {
        let mut wt_path: Option<String> = None;
        for line in stdout.lines() {
            if line.starts_with("worktree ") {
                wt_path = Some(line.strip_prefix("worktree ").unwrap().to_string());
            } else if line.starts_with("branch ") {
                let b = line.strip_prefix("branch ").unwrap().trim();
                if owned_branches.contains(&b.to_string()) {
                    let is_clean = repo
                        .run(&["status", "--porcelain"])
                        .map(|s| s.trim().is_empty())
                        .unwrap_or(true);
                    if !is_clean
                        && !options.detach_dirty_worktrees
                        && !options.snapshot_drafts
                        && !options.leave_worktrees
                    {
                        return Err(StaircaseError::Other(format!(
                            "worktree at {:?} attached to branch '{}' is dirty; use --detach-dirty-worktrees or --snapshot-drafts",
                            wt_path, b
                        )));
                    }
                    if !options.dry_run {
                        let step_cut = meta
                            .steps
                            .iter()
                            .find(|s| {
                                s.branch.as_deref() == Some(b)
                                    || s.branch.as_deref()
                                        == Some(b.strip_prefix("refs/heads/").unwrap_or(b))
                            })
                            .map(|s| s.cut.clone())
                            .unwrap_or_else(|| {
                                meta.steps.last().map(|s| s.cut.clone()).unwrap_or_default()
                            });
                        if !step_cut.is_empty() {
                            let _ = repo.run(&["checkout", "--detach", &step_cut]);
                        }
                    }
                }
            }
        }
    }

    let event_id = format!("evt-archive-{}", uuid::Uuid::new_v4().simple());
    let timestamp = current_timestamp();

    let mut archive_owned_refs = Vec::new();
    for (idx, full_ref) in owned_branches.iter().enumerate() {
        let ref_id = format!("owned-{}", idx + 1);
        let oid = repo.resolve_ref_opt(full_ref)?.unwrap_or_default();
        archive_owned_refs.push(ArchivedOwnedRef {
            ref_id: ref_id.clone(),
            original_refname: full_ref.clone(),
            object_type: "commit".to_string(),
            original_oid: oid,
            archive_refname: StaircaseRefs::archive_owned(&meta.id, &ref_id),
            ownership_class: "primary".to_string(),
            visibility_class: "hidden".to_string(),
            restoration_policy: "restore-or-rename".to_string(),
        });
    }

    let mut expected_source_oids = HashMap::new();
    let mut archive_retention_refs = HashMap::new();

    for step in &meta.steps {
        let step_key = if !step.id.is_empty() {
            &step.id
        } else {
            &step.name
        };
        expected_source_oids.insert(step_key.to_string(), step.cut.clone());
        archive_retention_refs.insert(
            step_key.to_string(),
            StaircaseRefs::archive_step(&meta.id, step_key),
        );
    }

    let archive_manifest = ArchiveManifest {
        archive_event_id: event_id.clone(),
        lineage_id: meta.id.clone(),
        archive_time: timestamp.clone(),
        actor: None,
        reason: options.reason.clone(),
        previous_record_oid: current_record.record_oid.clone(),
        canonical_name: meta.name.clone(),
        branch_layout_profile: meta.primary_branch_layout.clone(),
        branch_layout_base: meta.branch_layout_base.clone(),
        owned_refs: archive_owned_refs,
        expected_source_oids,
        archive_retention_refs,
        branch_configs: branch_configs.clone(),
        worktree_attachments: Vec::new(),
        draft_disposition: None,
        provider_disposition: None,
        name_reservation: true,
    };

    let mut updated_lifecycle = current_record.lifecycle.clone();
    updated_lifecycle.state = LifecycleState::Archived;
    updated_lifecycle.archive_reason = options.reason.clone();
    updated_lifecycle.name_reserved = true;
    updated_lifecycle.events.push(LifecycleEvent {
        event_id: event_id.clone(),
        kind: "archived".to_string(),
        timestamp,
        actor: None,
        record_oid_before: Some(current_record.record_oid.clone()),
        record_oid_after: None,
        canonical_name: Some(meta.name.clone()),
        reason: options.reason.clone(),
        details: serde_json::Value::Null,
    });

    let mut unowned_warnings = Vec::new();
    let cut_oids: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();

    if let Ok(stdout) = repo.run(&[
        "for-each-ref",
        "--format=%(refname) %(objectname)",
        "refs/heads/",
    ]) {
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                let refname = parts[0];
                let oid = parts[1];
                if !owned_branches.contains(&refname.to_string())
                    && cut_oids.contains(&oid.to_string())
                {
                    unowned_warnings.push(format!(
                        "warning: unowned branch {} still points to archived cut {}",
                        refname, oid
                    ));
                }
            }
        }
    }

    if options.dry_run {
        return Ok(ArchiveResult {
            archived_staircase_id: meta.id.clone(),
            canonical_name: meta.name.clone(),
            archive_event_id: event_id,
            moved_branches: owned_branches,
            unowned_warnings,
            is_dry_run: true,
        });
    }

    let record = persistence::write_record(
        repo,
        meta,
        &current_record.user_metadata,
        &updated_lifecycle,
        Some(&archive_manifest),
        true,
    )?;

    if let Some(event) = updated_lifecycle.events.last_mut() {
        event.record_oid_after = Some(record.record_oid.clone());
    }

    let public_ref = StaircaseRefs::public(&meta.name);
    let _ = repo.run(&["update-ref", "-d", &public_ref]);

    let state_prefix = format!("{}{}/", STATE_PREFIX, meta.id);
    if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", &state_prefix]) {
        for line in stdout.lines() {
            let r = line.trim();
            let _ = repo.run(&["update-ref", "-d", r]);
        }
    }

    for full_ref in &owned_branches {
        let b_name = full_ref.strip_prefix("refs/heads/").unwrap_or(full_ref);
        let _ = repo.run(&["config", "--remove-section", &format!("branch.{}", b_name)]);
        let _ = repo.run(&["update-ref", "-d", full_ref]);
    }

    Ok(ArchiveResult {
        archived_staircase_id: meta.id.clone(),
        canonical_name: meta.name.clone(),
        archive_event_id: event_id,
        moved_branches: owned_branches,
        unowned_warnings,
        is_dry_run: false,
    })
}

pub fn release_staircase_name(repo: &GitRepo, selector: &ResolvedSelector) -> Result<String> {
    let meta = selector.staircase.metadata();
    let archive_record_ref = StaircaseRefs::archive_record(&meta.id);

    let mut record = persistence::read_record(repo, &archive_record_ref)?;

    if record.lifecycle.state != LifecycleState::Archived {
        return Err(StaircaseError::Other(format!(
            "staircase '{}' is not archived; cannot release name",
            meta.name
        )));
    }

    record.lifecycle.name_reserved = false;
    let event_id = format!("evt-release-name-{}", uuid::Uuid::new_v4().simple());
    record.lifecycle.events.push(LifecycleEvent {
        event_id,
        kind: "name-released".to_string(),
        timestamp: current_timestamp(),
        actor: None,
        record_oid_before: Some(record.record_oid.clone()),
        record_oid_after: None,
        canonical_name: Some(meta.name.clone()),
        reason: None,
        details: serde_json::Value::Null,
    });

    if let Some(ref mut manifest) = record.archive_manifest {
        manifest.name_reservation = false;
    }

    let updated_record = persistence::write_record(
        repo,
        &record.metadata,
        &record.user_metadata,
        &record.lifecycle,
        record.archive_manifest.as_ref(),
        true,
    )?;

    Ok(updated_record.record_oid)
}

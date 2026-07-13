use super::restack::{RestackOptions, RestackStrategy, Restacker};
use crate::core::persistence;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{
    DraftAttachment, DraftClassification, DraftIntent, DraftSnapshot, RewriteMode, Step,
    WorktreeDraft,
};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftDiffMode {
    Staged,
    Unstaged,
    Combined,
    Untracked,
    Ignored,
}

#[derive(Debug, Clone, Default)]
pub struct MaterializeOptions {
    pub all_tracked: bool,
    pub include_untracked: bool,
    pub include_ignored: bool,
    pub allow_empty: bool,
    pub message: Option<String>,
    pub paths: Vec<String>,
    pub preserve_draft: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MaterializeResult {
    pub staircase_id: String,
    pub staircase_name: String,
    pub step_name: String,
    pub commit_oid: String,
    pub updated_steps_count: usize,
}

fn get_draft_attachment_path(repo: &GitRepo) -> Result<PathBuf> {
    Ok(repo.workdir.join(".git").join("staircase-draft.json"))
}

fn get_snapshots_dir(repo: &GitRepo) -> Result<PathBuf> {
    let p = repo.workdir.join(".git").join("staircase-snapshots");
    if !p.exists() {
        let _ = fs::create_dir_all(&p);
    }
    Ok(p)
}

fn detect_active_git_operation(repo: &GitRepo) -> Result<Option<String>> {
    let git_dir = repo.workdir.join(".git");
    let checks = [
        ("MERGE_HEAD", "merge"),
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
        ("BISECT_LOG", "bisect"),
    ];
    for (git_file, op_name) in checks {
        if git_dir.join(git_file).exists() {
            return Ok(Some(op_name.to_string()));
        }
    }
    Ok(None)
}

pub fn read_persistent_attachment(repo: &GitRepo) -> Result<Option<DraftAttachment>> {
    let file = get_draft_attachment_path(repo)?;
    if file.exists() {
        let content = fs::read_to_string(&file)?;
        if let Ok(att) = serde_json::from_str::<DraftAttachment>(&content) {
            return Ok(Some(att));
        }
    }
    Ok(None)
}

pub fn save_persistent_attachment(repo: &GitRepo, attachment: &DraftAttachment) -> Result<()> {
    let file = get_draft_attachment_path(repo)?;
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(attachment)?;
    fs::write(file, content)?;
    Ok(())
}

pub fn delete_persistent_attachment(repo: &GitRepo) -> Result<()> {
    let file = get_draft_attachment_path(repo)?;
    if file.exists() {
        let _ = fs::remove_file(file);
    }
    Ok(())
}

pub fn check_transient_operation(repo: &GitRepo) -> Result<Option<String>> {
    detect_active_git_operation(repo)
}

pub fn get_worktree_draft(repo: &GitRepo) -> Result<WorktreeDraft> {
    let basis = repo
        .resolve_commit_opt("HEAD")?
        .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());
    let head_branch = repo.current_branch()?;

    let transient_op = check_transient_operation(repo)?;

    let mut staged_paths = Vec::new();
    let mut unstaged_paths = Vec::new();
    let mut untracked_paths = Vec::new();
    let mut ignored_paths = Vec::new();
    let mut conflicted_paths = Vec::new();
    let mut is_submodule_dirty = false;

    let status_out = repo
        .command()
        .args(&["status", "--porcelain=v2", "--ignored=matching"])
        .check_status(false)
        .run()?;

    for line in status_out.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "1" | "2" => {
                if parts.len() >= 9 {
                    let xy = parts[1];
                    let sub = parts[2];
                    if sub.starts_with('S') && sub != "S..." {
                        is_submodule_dirty = true;
                    }
                    let path = parts.last().unwrap().to_string();
                    let chars: Vec<char> = xy.chars().collect();
                    if chars.len() >= 2 {
                        if chars[0] != '.' {
                            staged_paths.push(path.clone());
                        }
                        if chars[1] != '.' {
                            unstaged_paths.push(path);
                        }
                    }
                }
            }
            "u" => {
                if parts.len() >= 11 {
                    let path = parts.last().unwrap().to_string();
                    conflicted_paths.push(path);
                }
            }
            "?" => {
                if parts.len() >= 2 {
                    untracked_paths.push(parts[1..].join(" "));
                }
            }
            "!" => {
                if parts.len() >= 2 {
                    ignored_paths.push(parts[1..].join(" "));
                }
            }
            _ => {}
        }
    }

    let staged_tree_oid = if conflicted_paths.is_empty() {
        repo.run(&["write-tree"]).ok()
    } else {
        None
    };

    let persistent = read_persistent_attachment(repo)?;
    let mut is_attachment_stale = false;
    let mut attachment = None;

    if let Some(att) = persistent {
        if att.expected_basis != basis {
            is_attachment_stale = true;
        }
        attachment = Some(att);
    } else if basis != "0000000000000000000000000000000000000000" {
        // Automatic draft attachment: exact basis equality (Section 5.1)
        let all_staircases = persistence::list_staircases(repo)?;
        let mut matching_steps = Vec::new();

        for s in &all_staircases {
            for step in &s.steps {
                if step.cut == basis {
                    matching_steps.push((s.clone(), step.clone()));
                }
            }
        }

        if matching_steps.len() == 1 {
            let (staircase, step) = &matching_steps[0];
            attachment = Some(DraftAttachment {
                staircase_id: Some(staircase.id.clone()),
                staircase_name: Some(staircase.name.clone()),
                step_id: Some(step.id.clone()),
                step_name: Some(step.name.clone()),
                intent: DraftIntent::ExtendStep,
                expected_basis: basis.clone(),
                worktree_identity: repo.workdir.to_string_lossy().to_string(),
            });
        }
    }

    let classification = if transient_op.is_some() {
        DraftClassification::TransientOperation
    } else if !conflicted_paths.is_empty() {
        DraftClassification::Conflicted
    } else if is_submodule_dirty {
        DraftClassification::SubmoduleDirty
    } else if !staged_paths.is_empty() && !unstaged_paths.is_empty() {
        DraftClassification::PartiallyStaged
    } else if !staged_paths.is_empty() {
        DraftClassification::StagedOnly
    } else if !unstaged_paths.is_empty() {
        DraftClassification::UnstagedOnly
    } else if !untracked_paths.is_empty() {
        DraftClassification::Untracked
    } else {
        DraftClassification::Clean
    };

    Ok(WorktreeDraft {
        basis,
        head_branch,
        staged_paths,
        staged_tree_oid,
        unstaged_paths,
        untracked_paths,
        ignored_paths,
        conflicted_paths,
        transient_operation: transient_op,
        is_submodule_dirty,
        attachment,
        classification,
        is_attachment_stale,
    })
}

pub fn attach_draft(
    repo: &GitRepo,
    staircase_name: &str,
    step_name: Option<&str>,
    intent: Option<DraftIntent>,
) -> Result<DraftAttachment> {
    let rs = crate::core::resolve_staircase(repo, staircase_name, None)?.ok_or_else(|| {
        StaircaseError::Other(format!("Staircase '{}' not found", staircase_name))
    })?;

    let meta = if rs.is_managed() {
        rs.metadata().clone()
    } else {
        // Automatically adopt implicit staircase when persistently attached (Section 6.2)
        let adopted = crate::core::adopt(repo, rs.metadata())?;
        adopted
    };

    let selected_step = if let Some(sn) = step_name {
        meta.steps
            .iter()
            .find(|s| s.name == sn || s.id == sn)
            .cloned()
            .ok_or_else(|| StaircaseError::Other(format!("Step '{}' not found in staircase", sn)))?
    } else {
        meta.steps
            .last()
            .cloned()
            .ok_or_else(|| StaircaseError::Other("Staircase has no steps".to_string()))?
    };

    let basis = repo.resolve_commit("HEAD")?;

    let attachment = DraftAttachment {
        staircase_id: Some(meta.id.clone()),
        staircase_name: Some(meta.name.clone()),
        step_id: Some(selected_step.id.clone()),
        step_name: Some(selected_step.name.clone()),
        intent: intent.unwrap_or(DraftIntent::ExtendStep),
        expected_basis: basis,
        worktree_identity: repo.workdir.to_string_lossy().to_string(),
    };

    save_persistent_attachment(repo, &attachment)?;
    Ok(attachment)
}

pub fn detach_draft(repo: &GitRepo) -> Result<()> {
    delete_persistent_attachment(repo)
}

pub fn diff_draft(repo: &GitRepo, mode: DraftDiffMode) -> Result<String> {
    match mode {
        DraftDiffMode::Staged => repo.run(&["diff", "--cached"]),
        DraftDiffMode::Unstaged => repo.run(&["diff"]),
        DraftDiffMode::Combined => repo.run(&["diff", "HEAD"]),
        DraftDiffMode::Untracked => {
            let draft = get_worktree_draft(repo)?;
            if draft.untracked_paths.is_empty() {
                Ok("No untracked files.".to_string())
            } else {
                Ok(draft
                    .untracked_paths
                    .iter()
                    .map(|p| format!("Untracked: {}", p))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
        }
        DraftDiffMode::Ignored => {
            let draft = get_worktree_draft(repo)?;
            if draft.ignored_paths.is_empty() {
                Ok("No ignored files.".to_string())
            } else {
                Ok(draft
                    .ignored_paths
                    .iter()
                    .map(|p| format!("Ignored: {}", p))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
        }
    }
}

pub fn materialize_draft(
    repo: &GitRepo,
    staircase_target: Option<&str>,
    requested_intent: Option<DraftIntent>,
    options: &MaterializeOptions,
) -> Result<MaterializeResult> {
    let draft = get_worktree_draft(repo)?;

    if draft.classification == DraftClassification::Conflicted {
        return Err(StaircaseError::Other(
            "Cannot materialize: index contains unmerged conflicts".to_string(),
        ));
    }
    if draft.transient_operation.is_some() {
        return Err(StaircaseError::Other(
            "Cannot materialize: active Git operation in progress".to_string(),
        ));
    }

    if options.include_ignored {
        repo.run(&["add", "-f", "-A"])?;
    } else if options.include_untracked {
        repo.run(&["add", "-A"])?;
    } else if options.all_tracked {
        repo.run(&["add", "-u"])?;
    } else if !options.paths.is_empty() {
        let mut args = vec!["add", "--"];
        for p in &options.paths {
            args.push(p.as_str());
        }
        repo.run(&args)?;
    }

    let staged_tree = repo.run(&["write-tree"])?.trim().to_string();
    let basis_tree = repo.run(&["rev-parse", "HEAD^{tree}"])?.trim().to_string();

    if staged_tree == basis_tree && !options.allow_empty {
        return Err(StaircaseError::Other(
            "No staged changes to materialize. Use --allow-empty to create an empty commit."
                .to_string(),
        ));
    }

    let target_name = staircase_target
        .or_else(|| {
            draft
                .attachment
                .as_ref()
                .and_then(|a| a.staircase_name.as_deref())
        })
        .or_else(|| draft.head_branch.as_deref());

    let target_name = target_name.ok_or_else(|| {
        StaircaseError::Other(
            "No staircase target specified, attached, or inferred from current branch".to_string(),
        )
    })?;

    let rs = crate::core::resolve_staircase(repo, target_name, None)?
        .ok_or_else(|| StaircaseError::Other(format!("Staircase '{}' not found", target_name)))?;

    let mut meta = if rs.is_managed() {
        rs.metadata().clone()
    } else {
        crate::core::adopt(repo, rs.metadata())?
    };

    let intent = requested_intent
        .or_else(|| draft.attachment.as_ref().map(|a| a.intent.clone()))
        .unwrap_or(DraftIntent::ExtendStep);

    let (step_idx, target_step) = if let Some(att) = &draft.attachment {
        if let Some(sid) = &att.step_id {
            if let Some(pos) = meta
                .steps
                .iter()
                .position(|s| &s.id == sid || &s.name == sid)
            {
                (pos, meta.steps[pos].clone())
            } else {
                let pos = meta.steps.len() - 1;
                (pos, meta.steps[pos].clone())
            }
        } else {
            let pos = meta.steps.len() - 1;
            (pos, meta.steps[pos].clone())
        }
    } else {
        let pos = meta.steps.len() - 1;
        (pos, meta.steps[pos].clone())
    };

    let original_cuts: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();
    let basis_oid = draft.basis.clone();

    let commit_msg = options
        .message
        .clone()
        .unwrap_or_else(|| format!("Materialize draft for step '{}'", target_step.name));

    let commit_oid = repo
        .run(&[
            "commit-tree",
            &staged_tree,
            "-p",
            &basis_oid,
            "-m",
            &commit_msg,
        ])?
        .trim()
        .to_string();

    let mut updated_steps_count = 1;

    let restacker = Restacker::prepare(repo, &meta.steps)?;

    match intent {
        DraftIntent::ExtendStep | DraftIntent::Unassigned => {
            meta.steps[step_idx].cut = commit_oid.clone();
            if let Some(ref branch) = meta.steps[step_idx].branch {
                repo.update_branch(branch, &commit_oid)?;
            }
            repo.update_step_ref(&meta.id, &meta.steps[step_idx].id, &commit_oid)?;

            if step_idx + 1 < meta.steps.len() {
                let mut old_parents = Vec::new();
                for i in (step_idx + 1)..meta.steps.len() {
                    old_parents.push(original_cuts[i - 1].clone());
                }

                let mut remaining_steps = meta.steps[step_idx + 1..].to_vec();
                restacker.perform_restack(
                    &meta.id,
                    &mut remaining_steps,
                    &commit_oid,
                    &old_parents,
                    &RestackOptions {
                        strategy: RestackStrategy::Manual,
                        leave_upper_steps_stale: false,
                    },
                )?;

                for (i, step) in remaining_steps.into_iter().enumerate() {
                    meta.steps[step_idx + 1 + i] = step;
                    updated_steps_count += 1;
                }
            }

            if let Some(ref b) = draft.head_branch {
                let current_target = meta
                    .steps
                    .iter()
                    .find(|s| s.branch.as_deref() == Some(b.as_str()));
                if let Some(target_step) = current_target {
                    let target_oid = &target_step.cut;
                    repo.run(&["reset", "--soft", target_oid])?;
                } else {
                    repo.run(&["reset", "--soft", &commit_oid])?;
                }
            } else {
                repo.run(&["reset", "--soft", &commit_oid])?;
            }
        }
        DraftIntent::NewStep => {
            let new_step_name = format!("{}-step", target_step.name);
            let new_step_id = uuid::Uuid::new_v4().to_string();
            let new_step = Step {
                id: new_step_id.clone(),
                name: new_step_name.clone(),
                cut: commit_oid.clone(),
                branch: Some(new_step_name.clone()),
            };

            meta.steps.insert(step_idx + 1, new_step);
            repo.update_branch(&new_step_name, &commit_oid)?;
            repo.update_step_ref(&meta.id, &new_step_id, &commit_oid)?;

            if step_idx + 2 < meta.steps.len() {
                let mut old_parents = Vec::new();
                for i in (step_idx + 2)..meta.steps.len() {
                    // For NewStep, the old parent of the first restacked step (step_idx + 2)
                    // is the old cut of step_idx + 1.
                    old_parents.push(original_cuts[i - 2].clone());
                }
                // Wait, original_cuts is indexed by the OLD meta.steps.
                // meta.steps[step_idx+2] is OLD meta.steps[step_idx+1].
                // So its old parent was OLD meta.steps[step_idx].
                // Let's re-calculate more carefully.
            }
            // Actually, NewStep implementation in original was also a bit complex.
            // Let's just fix it properly.

            // Re-implementing restack for NewStep using perform_restack
            if step_idx + 2 < meta.steps.len() {
                let mut old_parents = Vec::new();
                // original_cuts[j] is the cut of OLD step j.
                // We inserted a new step at step_idx + 1.
                // So meta.steps[step_idx + 2] is OLD step step_idx + 1.
                // Its old parent was OLD step step_idx.
                for i in (step_idx + 2)..meta.steps.len() {
                    let old_idx = i - 1; // Index in OLD meta.steps
                    old_parents.push(original_cuts[old_idx - 1].clone());
                }

                let mut remaining_steps = meta.steps[step_idx + 2..].to_vec();
                restacker.perform_restack(
                    &meta.id,
                    &mut remaining_steps,
                    &commit_oid, // Base for first restacked step (which is NewStep)
                    // Wait, base for first restacked step should be the NEWLY inserted step's cut.
                    &old_parents,
                    &RestackOptions {
                        strategy: RestackStrategy::Manual,
                        leave_upper_steps_stale: false,
                    },
                )?;
                // wait, if I use commit_oid as base, it's correct because the first restacked step
                // is meta.steps[step_idx+2], and its parent is meta.steps[step_idx+1] which has cut commit_oid.

                for (i, step) in remaining_steps.into_iter().enumerate() {
                    meta.steps[step_idx + 2 + i] = step;
                    updated_steps_count += 1;
                }
            }

            repo.run(&["reset", "--soft", &commit_oid])?;
        }
        DraftIntent::RewriteStep(mode) => match mode {
            RewriteMode::Amend => {
                meta.steps[step_idx].cut = commit_oid.clone();
                if let Some(ref branch) = meta.steps[step_idx].branch {
                    repo.update_branch(branch, &commit_oid)?;
                }
                repo.update_step_ref(&meta.id, &meta.steps[step_idx].id, &commit_oid)?;
                repo.run(&["reset", "--soft", &commit_oid])?;
            }
            RewriteMode::Fixup | RewriteMode::FoldInto(_) => {
                meta.steps[step_idx].cut = commit_oid.clone();
                if let Some(ref branch) = meta.steps[step_idx].branch {
                    repo.update_branch(branch, &commit_oid)?;
                }
                repo.update_step_ref(&meta.id, &meta.steps[step_idx].id, &commit_oid)?;
                repo.run(&["reset", "--soft", &commit_oid])?;
            }
        },
    }

    persistence::write_metadata(repo, &meta)?;

    if let Some(mut att) = draft.attachment {
        att.expected_basis = commit_oid.clone();
        save_persistent_attachment(repo, &att)?;
    }

    Ok(MaterializeResult {
        staircase_id: meta.id,
        staircase_name: meta.name,
        step_name: meta.steps[step_idx].name.clone(),
        commit_oid,
        updated_steps_count,
    })
}

pub fn create_snapshot(repo: &GitRepo, _name: Option<&str>) -> Result<DraftSnapshot> {
    let draft = get_worktree_draft(repo)?;
    let snapshot_id = uuid::Uuid::new_v4().to_string();
    let created_at = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_secs().to_string(),
        Err(_) => "0".to_string(),
    };

    let staged_tree = draft.staged_tree_oid.clone();

    let worktree_patch = repo.run(&["diff"]).ok();

    let snapshot = DraftSnapshot {
        id: snapshot_id.clone(),
        created_at,
        basis: draft.basis.clone(),
        staged_tree,
        worktree_tree: worktree_patch,
        untracked_paths: draft.untracked_paths.clone(),
        attachment: draft.attachment.clone(),
    };

    let dir = get_snapshots_dir(repo)?;
    let snapshot_file = dir.join(format!("{}.json", snapshot_id));
    let content = serde_json::to_string_pretty(&snapshot)?;
    fs::write(snapshot_file, content)?;

    Ok(snapshot)
}

pub fn restore_snapshot(repo: &GitRepo, snapshot_id: &str) -> Result<DraftSnapshot> {
    let dir = get_snapshots_dir(repo)?;
    let snapshot_file = dir.join(format!("{}.json", snapshot_id));
    if !snapshot_file.exists() {
        return Err(StaircaseError::Other(format!(
            "Snapshot '{}' not found",
            snapshot_id
        )));
    }

    let content = fs::read_to_string(&snapshot_file)?;
    let snapshot: DraftSnapshot = serde_json::from_str(&content)?;

    let current_draft = get_worktree_draft(repo)?;
    if current_draft.classification != DraftClassification::Clean {
        return Err(StaircaseError::Other(
            "Restoration collision: current worktree draft is not clean".to_string(),
        ));
    }

    if let Some(ref patch) = snapshot.worktree_tree {
        let _ = repo.run_with_stdin(&["apply"], patch);
    }

    if let Some(ref att) = snapshot.attachment {
        save_persistent_attachment(repo, att)?;
    }

    Ok(snapshot)
}

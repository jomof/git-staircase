use super::persistence;
use super::refs::{ARCHIVE_PREFIX, STATE_PREFIX, StaircaseRefs};
use crate::core::ResolvedStaircase;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{LandingPolicy, LifecycleState, StaircaseMetadata, Step};
use std::collections::HashSet;
use uuid::Uuid;

pub struct RebaseOptions {
    pub leave_upper_steps_stale: bool,
}

pub struct ReorderOptions {
    pub no_restack: bool,
}

pub struct DropOptions {
    pub restack: bool,
    pub leave_descendants_stale: bool,
}

pub struct SplitOptions {
    pub no_ref: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinRefAction {
    Delete,
    Rename(String),
    Keep,
}

pub struct JoinOptions {
    pub ref_action: JoinRefAction,
}

fn check_active(staircase: &ResolvedStaircase) -> Result<()> {
    if staircase
        .metadata()
        .lifecycle
        .as_ref()
        .map(|l| l.state == LifecycleState::Archived)
        .unwrap_or(false)
    {
        return Err(StaircaseError::Other(
            "staircase is archived; unarchive it before mutation".to_string(),
        ));
    }
    Ok(0).map(|_| ())
}

pub fn split(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    at_commit: &str,
    new_step_name: Option<&str>,
    options: SplitOptions,
) -> Result<()> {
    check_active(staircase)?;
    let at_oid = validate_split(repo, staircase, step_index, at_commit)?;

    let name = match new_step_name {
        Some(n) => n.to_string(),
        None => format!("{}-split", staircase.metadata().steps[step_index].name),
    };

    let new_step = Step {
        id: Uuid::new_v4().to_string(),
        name: name.clone(),
        cut: at_oid.clone(),
        branch: if options.no_ref {
            None
        } else if staircase.is_managed() {
            None
        } else {
            new_step_name.map(|n| n.to_string())
        },
    };

    staircase.add_step(repo, step_index, new_step, options.no_ref)?;
    Ok(())
}

pub fn validate_split(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    at_commit: &str,
) -> Result<String> {
    if step_index >= staircase.metadata().steps.len() {
        return Err(StaircaseError::InvalidStructure(format!(
            "Step index {} out of bounds",
            step_index
        )));
    }
    let at_oid = repo.resolve_commit(at_commit)?;
    let cut_oid = &staircase.metadata().steps[step_index].cut;
    let prev_cut_oid = if step_index == 0 {
        repo.resolve_commit(&staircase.metadata().target)?
    } else {
        staircase.metadata().steps[step_index - 1].cut.clone()
    };
    if !repo.is_ancestor(&prev_cut_oid, &at_oid)? {
        return Err(StaircaseError::InvalidStructure(format!(
            "Commit {} is not a descendant of previous step cut {}",
            at_commit, prev_cut_oid
        )));
    }
    if !repo.is_ancestor(&at_oid, cut_oid)? {
        return Err(StaircaseError::InvalidStructure(format!(
            "Commit {} is not an ancestor of step cut {}",
            at_commit, cut_oid
        )));
    }
    if at_oid == prev_cut_oid || at_oid == *cut_oid {
        return Err(StaircaseError::InvalidStructure(
            "Cannot split at step boundaries".to_string(),
        ));
    }
    Ok(at_oid)
}

pub fn validate_split_plan(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    at_commit: &str,
    new_step_name: Option<&str>,
    no_ref: bool,
) -> Result<()> {
    let at_oid = validate_split(repo, staircase, step_index, at_commit)?;
    let step = Step {
        id: Uuid::new_v4().to_string(),
        name: new_step_name
            .map(str::to_string)
            .unwrap_or_else(|| format!("{}-split", staircase.metadata().steps[step_index].name)),
        cut: at_oid,
        branch: new_step_name.map(str::to_string),
    };
    let mut metadata = staircase.metadata().clone();
    if no_ref {
        metadata.primary_branch_layout = None;
        metadata.branch_layout_base = None;
    }
    metadata.steps.insert(step_index, step);
    super::resolved::validate_renumbering(repo, staircase.metadata(), &mut metadata, true)
}

pub fn join(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index_1: usize,
    step_index_2: usize,
    options: JoinOptions,
) -> Result<()> {
    check_active(staircase)?;
    let (low, high) = if step_index_1 < step_index_2 {
        (step_index_1, step_index_2)
    } else {
        (step_index_2, step_index_1)
    };

    if low + 1 != high {
        return Err(StaircaseError::InvalidStructure(format!(
            "Steps to join must be adjacent: {} and {}",
            low, high
        )));
    }

    if high >= staircase.metadata().steps.len() {
        return Err(StaircaseError::InvalidStructure(format!(
            "Step index {} out of bounds",
            high
        )));
    }

    let removed_step = staircase.metadata().steps[low].clone();
    if matches!(options.ref_action, JoinRefAction::Rename(_)) {
        return Err(StaircaseError::UnsupportedTopology {
            operation: "join".into(),
            reason: "renaming a retired boundary ref is not a canonical join operation".into(),
        });
    }
    if matches!(options.ref_action, JoinRefAction::Delete) {
        if let Some(branch) = &removed_step.branch {
            let reference = format!("refs/heads/{}", branch);
            let actual = repo.resolve_ref_opt(&reference)?;
            if actual.as_deref() != Some(removed_step.cut.as_str()) {
                return Err(StaircaseError::RefCollision {
                    reference,
                    expected: removed_step.cut.clone(),
                    actual: actual.unwrap_or_else(|| "<missing>".into()),
                });
            }
        }
    }

    let managed = if staircase.is_managed() {
        staircase.clone()
    } else {
        ResolvedStaircase::Managed(crate::core::adopt(repo, staircase.metadata())?)
    };
    let mut desired = managed.metadata().clone();
    desired.steps.remove(low);
    super::resolved::validate_renumbering(repo, managed.metadata(), &mut desired, true)?;
    let selector = super::resolved::ResolvedSelector {
        staircase: managed,
        step_index: None,
    };
    let record = persistence::read_record(
        repo,
        &StaircaseRefs::record(&desired.id, LifecycleState::Active),
    )?;
    let extras = if options.ref_action == JoinRefAction::Keep {
        removed_step
            .branch
            .as_ref()
            .map(|branch| {
                vec![(
                    format!("refs/heads/{}", branch),
                    Some(removed_step.cut.clone()),
                    Some(removed_step.cut.clone()),
                )]
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    super::local::publish_record_parts_extra(
        repo,
        &selector,
        desired,
        record.user_metadata,
        "join",
        false,
        &extras,
    )?;
    Ok(())
}

pub fn reorder(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    new_order: &[usize],
    options: ReorderOptions,
) -> Result<()> {
    reorder_internal(repo, staircase, new_order, options, true, false)
}

pub fn reorder_dry_run(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    new_order: &[usize],
    options: ReorderOptions,
) -> Result<()> {
    reorder_internal(repo, staircase, new_order, options, true, true)
}

fn reorder_internal(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    new_order: &[usize],
    options: ReorderOptions,
    require_complete: bool,
    dry_run: bool,
) -> Result<()> {
    check_active(staircase)?;
    let mut metadata = staircase.metadata().clone();
    let old_steps = metadata.steps.clone();

    let mut seen = HashSet::new();
    for &idx in new_order {
        if idx >= old_steps.len() || !seen.insert(idx) {
            return Err(StaircaseError::Other(
                "Invalid step indices in new order".to_string(),
            ));
        }
    }
    if require_complete && new_order.len() != old_steps.len() {
        return Err(StaircaseError::InvalidStructure(
            "new order must be a complete permutation of all steps".into(),
        ));
    }

    let mut new_steps = Vec::new();
    for &idx in new_order {
        new_steps.push(old_steps[idx].clone());
    }

    metadata.steps = new_steps;
    super::resolved::validate_renumbering(
        repo,
        staircase.metadata(),
        &mut metadata,
        options.no_restack,
    )?;
    if options.no_restack {
        if dry_run {
            Ok(())
        } else {
            publish_metadata_common(repo, staircase, metadata, "reorder")
        }
    } else {
        ensure_rewrite_supported(repo, staircase.metadata(), "reorder")?;
        let groups = step_commit_groups(repo, staircase.metadata())?;
        let reordered = new_order
            .iter()
            .map(|index| groups[*index].clone())
            .collect::<Vec<_>>();
        let start_step = new_order
            .iter()
            .enumerate()
            .take_while(|(new_index, old_index)| *new_index == **old_index)
            .count();
        let target = if start_step == 0 {
            recorded_target(repo, &metadata)?
        } else {
            metadata.steps[start_step - 1].cut.clone()
        };
        super::rewrite::replay(
            repo,
            staircase,
            metadata,
            reordered[start_step..].to_vec(),
            start_step,
            target,
            "reorder",
            dry_run,
        )
    }
}

pub fn drop(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    options: DropOptions,
) -> Result<()> {
    drop_with_dry_run(repo, staircase, step_index, options, false)
}

pub fn drop_with_dry_run(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    options: DropOptions,
    dry_run: bool,
) -> Result<()> {
    check_active(staircase)?;
    let metadata = staircase.metadata().clone();
    if step_index >= metadata.steps.len() {
        return Err(StaircaseError::Other(
            "Step index out of bounds".to_string(),
        ));
    }

    let mut desired = metadata.clone();
    desired.steps.remove(step_index);
    super::resolved::validate_renumbering(
        repo,
        &metadata,
        &mut desired,
        !options.restack || options.leave_descendants_stale,
    )?;
    if !options.restack || options.leave_descendants_stale {
        return if dry_run {
            Ok(())
        } else {
            publish_metadata_common(repo, staircase, desired, "drop")
        };
    }
    ensure_rewrite_supported(repo, &metadata, "drop")?;
    let mut groups = step_commit_groups(repo, &metadata)?;
    groups.remove(step_index);
    let start_step = step_index.min(desired.steps.len());
    let target = if start_step == 0 {
        recorded_target(repo, &metadata)?
    } else {
        desired.steps[start_step - 1].cut.clone()
    };
    super::rewrite::replay(
        repo,
        staircase,
        desired,
        groups[start_step..].to_vec(),
        start_step,
        target,
        "drop",
        dry_run,
    )
}

pub fn move_commits(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    from_step_index: usize,
    to_step_index: usize,
    commits: &[String],
) -> Result<()> {
    move_commits_with_dry_run(
        repo,
        staircase,
        from_step_index,
        to_step_index,
        commits,
        false,
    )
}

pub fn move_commits_with_dry_run(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    from_step_index: usize,
    to_step_index: usize,
    commits: &[String],
    dry_run: bool,
) -> Result<()> {
    check_active(staircase)?;
    if commits.is_empty() {
        return Ok(());
    }
    let mut metadata = staircase.metadata().clone();
    if from_step_index >= metadata.steps.len() || to_step_index >= metadata.steps.len() {
        return Err(StaircaseError::Other(
            "Step index out of bounds".to_string(),
        ));
    }

    if from_step_index == to_step_index {
        return Ok(());
    }

    ensure_rewrite_supported(repo, &metadata, "move")?;
    let mut groups = step_commit_groups(repo, &metadata)?;
    let original_order = groups.iter().flatten().cloned().collect::<Vec<_>>();
    let selected = commits
        .iter()
        .map(|commit| repo.resolve_commit(commit))
        .collect::<Result<Vec<_>>>()?;
    let source = &groups[from_step_index];
    let mut positions = selected
        .iter()
        .map(|commit| {
            source
                .iter()
                .position(|candidate| candidate == commit)
                .ok_or_else(|| {
                    StaircaseError::InvalidStructure(format!(
                        "commit {} does not belong to source step '{}'",
                        commit, metadata.steps[from_step_index].name
                    ))
                })
        })
        .collect::<Result<Vec<_>>>()?;
    positions.sort_unstable();
    positions.dedup();
    if positions.len() != selected.len() {
        return Err(StaircaseError::InvalidStructure(
            "move commit selection contains duplicates".into(),
        ));
    }
    let moved = positions
        .iter()
        .map(|index| groups[from_step_index][*index].clone())
        .collect::<Vec<_>>();
    groups[from_step_index].retain(|commit| !selected.contains(commit));
    if groups[from_step_index].is_empty() {
        return Err(StaircaseError::UnsupportedTopology {
            operation: "move".into(),
            reason: "moving all commits would leave the source step empty; use drop/join".into(),
        });
    }
    groups[to_step_index].extend(moved);
    if groups.iter().flatten().cloned().collect::<Vec<_>>() == original_order {
        for (step, commits) in metadata.steps.iter_mut().zip(&groups) {
            step.cut = commits
                .last()
                .expect("empty source and target groups were rejected")
                .clone();
        }
        return if dry_run {
            Ok(())
        } else {
            publish_metadata_common(repo, staircase, metadata, "move")
        };
    }
    let target = recorded_target(repo, &metadata)?;
    super::rewrite::replay(
        repo, staircase, metadata, groups, 0, target, "move", dry_run,
    )
}

pub fn restack(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    options: RebaseOptions,
) -> Result<()> {
    check_active(staircase)?;
    let status = crate::core::status::get_status_metadata(
        repo,
        staircase.metadata().clone(),
        !staircase.is_managed(),
    )?;

    if status.is_clean {
        return Ok(());
    }

    let mut metadata = status.metadata.clone();
    ensure_rewrite_supported(repo, &metadata, "restack")?;
    let mut groups = Vec::new();
    let mut actual_predecessor = recorded_target(repo, &metadata)?;
    let mut recorded_predecessor = actual_predecessor.clone();
    let mut current_base = repo.resolve_commit(&metadata.target)?;
    let mut start_step = 0;
    for index in 0..metadata.steps.len() {
        let step = metadata.steps[index].clone();
        let actual = status.steps[index]
            .actual_oid
            .clone()
            .unwrap_or_else(|| step.cut.clone());

        let predecessor = if repo.is_ancestor(&actual_predecessor, &actual)? {
            &actual_predecessor
        } else if repo.is_ancestor(&recorded_predecessor, &actual)? {
            &recorded_predecessor
        } else {
            return Err(StaircaseError::UnsupportedTopology {
                operation: "restack".into(),
                reason: format!(
                    "actual tip of step '{}' does not descend from its recorded predecessor",
                    step.name
                ),
            });
        };

        if groups.is_empty() && repo.is_ancestor(&current_base, &actual)? {
            metadata.steps[index].cut = actual.clone();
            current_base = actual.clone();
            start_step = index + 1;
        } else {
            groups.push(repo.commits_between(predecessor, &actual)?);
        }
        actual_predecessor = actual;
        recorded_predecessor = step.cut;
    }
    if groups.is_empty() {
        return publish_metadata_common(repo, staircase, metadata, "restack");
    }
    if options.leave_upper_steps_stale {
        groups.truncate(1);
    }
    super::rewrite::replay(
        repo,
        staircase,
        metadata,
        groups,
        start_step,
        current_base,
        "restack",
        false,
    )
}

pub fn restack_from(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    from_step: usize,
    dry_run: bool,
) -> Result<()> {
    check_active(staircase)?;
    if from_step >= staircase.metadata().steps.len() {
        return Err(StaircaseError::InvalidStructure(
            "partial restack step is out of range".into(),
        ));
    }
    ensure_rewrite_supported(repo, staircase.metadata(), "restack")?;
    let status = crate::core::status::get_status_metadata(
        repo,
        staircase.metadata().clone(),
        !staircase.is_managed(),
    )?;
    let mut groups = Vec::new();
    for index in from_step..status.metadata.steps.len() {
        let predecessor = if index == 0 {
            recorded_target(repo, &status.metadata)?
        } else {
            status.metadata.steps[index - 1].cut.clone()
        };
        let actual = status.steps[index]
            .actual_oid
            .clone()
            .unwrap_or_else(|| status.metadata.steps[index].cut.clone());
        if !repo.is_ancestor(&predecessor, &actual)? {
            return Err(StaircaseError::UnsupportedTopology {
                operation: "restack".into(),
                reason: format!(
                    "step '{}' has diverged from its recorded predecessor",
                    status.metadata.steps[index].name
                ),
            });
        }
        groups.push(repo.commits_between(&predecessor, &actual)?);
    }
    let base = if from_step == 0 {
        repo.resolve_commit(&status.metadata.target)?
    } else {
        status.metadata.steps[from_step - 1].cut.clone()
    };
    super::rewrite::replay(
        repo,
        staircase,
        status.metadata,
        groups,
        from_step,
        base,
        "restack",
        dry_run,
    )
}

pub fn rebase(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    onto: &str,
    options: RebaseOptions,
) -> Result<()> {
    rebase_with_dry_run(repo, staircase, onto, options, false)
}

pub fn rebase_with_dry_run(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    onto: &str,
    options: RebaseOptions,
    dry_run: bool,
) -> Result<()> {
    check_active(staircase)?;
    let original = staircase.metadata();
    ensure_rewrite_supported(repo, original, "rebase")?;
    let groups = step_commit_groups(repo, original)?;
    let mut metadata = original.clone();
    metadata.target = repo
        .resolve_symbolic_full_name(onto)
        .unwrap_or_else(|_| onto.to_string());
    let groups = if options.leave_upper_steps_stale {
        groups.into_iter().take(1).collect()
    } else {
        groups
    };
    super::rewrite::replay(
        repo,
        staircase,
        metadata,
        groups,
        0,
        repo.resolve_commit(onto)?,
        "rebase",
        dry_run,
    )
}

fn ensure_rewrite_supported(
    repo: &GitRepo,
    metadata: &StaircaseMetadata,
    operation: &str,
) -> Result<()> {
    let mut predecessor = recorded_target(repo, metadata)?;
    for step in &metadata.steps {
        let merges = repo.has_merges(&predecessor, &step.cut)?;
        if let Some(commit) = merges {
            return Err(StaircaseError::UnsupportedTopology {
                operation: operation.into(),
                reason: format!(
                    "step '{}' contains merge commit {}; configure an explicit merge policy first",
                    step.name, commit
                ),
            });
        }
        predecessor = step.cut.clone();
    }
    Ok(())
}

fn step_commit_groups(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<Vec<Vec<String>>> {
    let mut predecessor = recorded_target(repo, metadata)?;
    let mut groups = Vec::new();
    for step in &metadata.steps {
        if !repo.is_ancestor(&predecessor, &step.cut)? {
            return Err(StaircaseError::UnsupportedTopology {
                operation: "rewrite".into(),
                reason: format!(
                    "step '{}' does not descend from its recorded predecessor",
                    step.name
                ),
            });
        }
        groups.push(repo.commits_between(&predecessor, &step.cut)?);
        predecessor = step.cut.clone();
    }
    Ok(groups)
}

fn recorded_target(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    metadata
        .user_metadata
        .as_ref()
        .and_then(|user| {
            user.extensions
                .get("git-staircase.internal.integration-anchor")
        })
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .map(Ok)
        .unwrap_or_else(|| repo.resolve_commit(&metadata.target))
}

fn publish_metadata_common(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    mut metadata: StaircaseMetadata,
    kind: &str,
) -> Result<()> {
    let managed = if staircase.is_managed() {
        staircase.clone()
    } else {
        ResolvedStaircase::Managed(super::resolved::adopt(repo, staircase.metadata())?)
    };
    metadata.id = managed.metadata().id.clone();
    for step in &mut metadata.steps {
        if step.id.is_empty() {
            if let Some(original) = managed.metadata().steps.iter().find(|original| {
                original.name == step.name
                    || (step.branch.is_some() && original.branch == step.branch)
            }) {
                step.id = original.id.clone();
            }
        }
    }
    let selector = super::resolved::ResolvedSelector {
        staircase: managed,
        step_index: None,
    };
    super::local::publish_metadata(repo, &selector, metadata, kind, false)?;
    Ok(())
}

pub fn delete(repo: &GitRepo, id: &str, delete_branches: bool) -> Result<()> {
    let metadata = persistence::read_metadata(repo, id)?;
    let mut plan = super::operation::MutationPlan::new("delete", Some(metadata.id.clone()));
    for prefix in [
        format!("{}{}/", STATE_PREFIX, metadata.id),
        format!("{}{}/", ARCHIVE_PREFIX, metadata.id),
    ] {
        let lines = repo.for_each_ref(&prefix, "%(refname) %(objectname)", None)?;
        for line in lines {
            if let Some((reference, oid)) = line.split_once(" ") {
                plan.update(reference, Some(oid.into()), None);
            }
        }
    }
    let public = StaircaseRefs::public(&metadata.name);
    if let Some(oid) = repo.resolve_ref_opt(&public)? {
        plan.update(public, Some(oid), None);
    }
    if delete_branches {
        for step in &metadata.steps {
            if let Some(branch) = &step.branch {
                let reference = format!("refs/heads/{}", branch);
                let actual = repo.resolve_ref_opt(&reference)?;
                if actual.as_deref() != Some(step.cut.as_str()) {
                    return Err(StaircaseError::RefCollision {
                        reference,
                        expected: step.cut.clone(),
                        actual: actual.unwrap_or_else(|| "<missing>".into()),
                    });
                }
                plan.update(reference, actual, None);
            }
        }
    }
    plan.publish(repo, false)?;
    Ok(())
}

pub struct LandOptions {
    pub policy: Option<LandingPolicy>,
}

pub fn land(repo: &GitRepo, staircase: &ResolvedStaircase, options: LandOptions) -> Result<()> {
    check_active(staircase)?;
    let status = crate::core::status::get_status_metadata(
        repo,
        staircase.metadata().clone(),
        !staircase.is_managed(),
    )?;

    if !status.is_clean {
        return Err(StaircaseError::Other(
            "Staircase is stale or modified. Please run restack or update metadata before landing."
                .to_string(),
        ));
    }

    let metadata = &status.metadata;
    let policy = options
        .policy
        .or(metadata.landing_policy)
        .unwrap_or(LandingPolicy::Stepwise);

    let top_cut = &metadata
        .steps
        .last()
        .ok_or_else(|| StaircaseError::InvalidStructure("Empty staircase".to_string()))?
        .cut;

    if metadata.target.starts_with("refs/") {
        let expected = repo.resolve_ref_opt(&metadata.target)?;
        let mut plan = super::operation::MutationPlan::new("land", Some(metadata.id.clone()));
        let _ = policy;
        plan.update(metadata.target.clone(), expected, Some(top_cut.to_string()));
        plan.publish(repo, false)?;
    } else {
        return Err(StaircaseError::Other(format!(
            "Target {} is not a ref, cannot land",
            metadata.target
        )));
    }

    Ok(())
}

pub fn land_through(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    through_step: usize,
    dry_run: bool,
) -> Result<()> {
    check_active(staircase)?;
    let managed = if staircase.is_managed() {
        staircase.clone()
    } else if dry_run {
        staircase.clone()
    } else {
        ResolvedStaircase::Managed(super::resolved::adopt(repo, staircase.metadata())?)
    };
    let metadata = managed.metadata();
    if through_step >= metadata.steps.len() {
        return Err(StaircaseError::InvalidStructure(
            "landing step is out of range".into(),
        ));
    }
    if through_step + 1 == metadata.steps.len() {
        return Err(StaircaseError::UnsupportedTopology {
            operation: "partial-land".into(),
            reason: "the selected prefix is the whole staircase; use aggregate or stepwise land"
                .into(),
        });
    }
    let status =
        crate::core::status::get_status_metadata(repo, metadata.clone(), !managed.is_managed())?;
    if !status.is_clean {
        return Err(StaircaseError::UnsupportedTopology {
            operation: "partial-land".into(),
            reason: "partial landing requires a clean staircase".into(),
        });
    }
    let target_ref = repo
        .resolve_symbolic_full_name(&metadata.target)
        .unwrap_or_else(|_| metadata.target.clone());
    if !target_ref.starts_with("refs/") {
        return Err(StaircaseError::InvalidStructure(
            "partial landing requires a symbolic target ref".into(),
        ));
    }
    let old_target = repo.resolve_ref_opt(&target_ref)?;
    let landed_cut = metadata.steps[through_step].cut.clone();
    let newly_landed_ids = metadata.steps[..=through_step]
        .iter()
        .map(|step| step.id.clone())
        .collect::<Vec<_>>();
    let mut desired = metadata.clone();
    desired.steps.drain(..=through_step);
    super::resolved::validate_renumbering(repo, metadata, &mut desired, true)?;
    if dry_run {
        return Ok(());
    }
    let selector = super::resolved::ResolvedSelector {
        staircase: managed,
        step_index: None,
    };
    let record = persistence::read_record(
        repo,
        &StaircaseRefs::record(&desired.id, LifecycleState::Active),
    )?;
    let mut user_metadata = record.user_metadata;
    user_metadata.extensions.insert(
        "git-staircase.internal.integration-anchor".into(),
        serde_json::Value::String(landed_cut.clone()),
    );
    let mut landed_step_ids = user_metadata
        .extensions
        .get("git-staircase.internal.structural-state")
        .and_then(|state| state.get("landed_step_ids"))
        .and_then(|ids| ids.as_array())
        .cloned()
        .unwrap_or_default();
    landed_step_ids.extend(newly_landed_ids.into_iter().map(serde_json::Value::String));
    user_metadata.extensions.insert(
        "git-staircase.internal.structural-state".into(),
        serde_json::json!({
            "kind": "partially-landed",
            "landed_step_ids": landed_step_ids,
        }),
    );
    super::local::publish_record_parts_extra(
        repo,
        &selector,
        desired,
        user_metadata,
        "partial-land",
        false,
        &[(target_ref, old_target, Some(landed_cut))],
    )?;
    Ok(())
}

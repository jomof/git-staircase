use crate::core::operation::{
    MutationPlan, OperationJournal, OperationPhase, OperationResult, begin_resumable,
    publish_resumable, restore_draft_after_success,
};
use crate::core::persistence;
use crate::core::refs::StaircaseRefs;
use crate::core::resolved::{ResolvedStaircase, adopt};
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{LifecycleState, StaircaseLifecycle, StaircaseMetadata, StaircaseUserMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RewriteContinuation {
    schema: String,
    version: u32,
    kind: String,
    old_record_oid: Option<String>,
    original: StaircaseMetadata,
    desired: StaircaseMetadata,
    user_metadata: StaircaseUserMetadata,
    lifecycle: StaircaseLifecycle,
    commit_groups: Vec<Vec<String>>,
    start_step: usize,
    end_step: usize,
    next_step: usize,
    next_commit: usize,
    current_base: String,
    pending_cherry_pick: bool,
    ensure_change_ids: bool,
}

pub(crate) fn replay(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    desired: StaircaseMetadata,
    commit_groups: Vec<Vec<String>>,
    start_step: usize,
    base_oid: String,
    kind: &str,
    dry_run: bool,
    ensure_change_ids: bool,
) -> Result<()> {
    if start_step > desired.steps.len() || start_step + commit_groups.len() > desired.steps.len() {
        return Err(StaircaseError::InvalidStructure(
            "rewrite plan has inconsistent step ranges".into(),
        ));
    }
    for (offset, commits) in commit_groups.iter().enumerate() {
        if commits.is_empty() {
            return Err(StaircaseError::UnsupportedTopology {
                operation: kind.into(),
                reason: format!(
                    "rewrite would leave step '{}' empty",
                    desired.steps[start_step + offset].name
                ),
            });
        }
        for commit in commits {
            let parents = repo
                .run(&["rev-list", "--parents", "-n", "1", commit])?
                .split_whitespace()
                .count()
                .saturating_sub(1);
            if parents != 1 {
                return Err(StaircaseError::UnsupportedTopology {
                    operation: kind.into(),
                    reason: format!(
                        "commit {} has {} parents; only single-parent commits can be replayed",
                        commit, parents
                    ),
                });
            }
        }
    }
    if dry_run {
        return Ok(());
    }

    let should_adopt = if staircase.is_managed() {
        false
    } else {
        // According to Spec 8.3, ideally remain implicit if result is clean.
        // For now, we only avoid adoption for rebase and restack if not forced.
        !matches!(kind, "rebase" | "restack")
    };

    let managed = if staircase.is_managed() {
        staircase.clone()
    } else if should_adopt {
        ResolvedStaircase::Managed(adopt(repo, staircase.metadata())?)
    } else {
        staircase.clone()
    };

    let mut desired = desired;
    desired.id = managed.metadata().id.clone();
    desired.name = managed.metadata().name.clone();
    for (old, new) in managed
        .metadata()
        .steps
        .iter()
        .zip(desired.steps.iter_mut())
    {
        if new.id.is_empty() {
            new.id = old.id.clone();
        }
    }
    let record_ref = StaircaseRefs::record(&desired.id, LifecycleState::Active);
    let old_record_oid = repo.resolve_ref_opt(&record_ref)?;
    let (mut old_metadata, mut old_user_metadata, old_lifecycle) =
        if let Some(ref oid) = old_record_oid {
            let rec = persistence::read_record(repo, oid)?;
            (rec.metadata, rec.user_metadata, rec.lifecycle)
        } else {
            (
                managed.metadata().clone(),
                StaircaseUserMetadata::default(),
                StaircaseLifecycle {
                    state: LifecycleState::Active,
                    ..Default::default()
                },
            )
        };
    old_metadata.name = managed.metadata().name.clone();
    ensure_worktree_safety(repo, &old_metadata, &desired)?;
    let lease_plan = lease_plan(
        repo,
        &old_metadata,
        &desired,
        old_record_oid.as_deref(),
        kind,
    )?;
    let end_step = start_step + commit_groups.len();
    if start_step == 0 && matches!(kind, "rebase" | "restack") {
        old_user_metadata.extensions.insert(
            "git-staircase.internal.integration-anchor".into(),
            serde_json::Value::String(base_oid.clone()),
        );
    }
    let continuation = RewriteContinuation {
        schema: "git-staircase/rewrite-continuation".into(),
        version: 1,
        kind: kind.into(),
        old_record_oid,
        original: old_metadata,
        desired,
        user_metadata: old_user_metadata,
        lifecycle: old_lifecycle,
        commit_groups,
        start_step,
        end_step,
        next_step: start_step,
        next_commit: 0,
        current_base: base_oid,
        pending_cherry_pick: false,
        ensure_change_ids,
    };
    let mut journal = begin_resumable(repo, &lease_plan, serde_json::to_value(&continuation)?)?;
    repo.run(&["reset", "--hard", "HEAD"])?;
    repo.run(&["checkout", "--detach", &continuation.current_base])?;
    run(repo, &mut journal)
}

pub(crate) fn continue_operation(
    repo: &GitRepo,
    mut journal: OperationJournal,
) -> Result<OperationResult> {
    let mut state = read_state(&journal)?;
    if state.pending_cherry_pick {
        if crate::core::draft::check_transient_operation(repo)?.as_deref() == Some("cherry-pick") {
            if let Err(error) = repo
                .command()
                .args(["cherry-pick", "--continue"])
                .env("GIT_EDITOR", "true")
                .run()
            {
                journal.phase = OperationPhase::Paused;
                journal.persist(repo)?;
                return Err(error);
            }
        } else if repo.resolve_commit("HEAD")? == state.current_base {
            journal.phase = OperationPhase::Paused;
            journal.persist(repo)?;
            return Err(StaircaseError::ExternalOperation {
                operation: "cherry-pick".into(),
                owner: "git staircase continue|abort".into(),
            });
        }
        state.current_base = repo.resolve_commit("HEAD")?;
        state.next_commit += 1;
        state.pending_cherry_pick = false;
        save_state(repo, &mut journal, &state)?;
    }
    run(repo, &mut journal)?;
    Ok(OperationResult {
        schema: "git-staircase/operation-result".into(),
        version: 1,
        operation_id: journal.operation_id,
        kind: journal.kind,
        transition: "continued".into(),
        restored_refs: 0,
        draft_restored: true,
    })
}

fn run(repo: &GitRepo, journal: &mut OperationJournal) -> Result<()> {
    let mut state = read_state(journal)?;
    while state.next_step < state.end_step {
        let group_index = state.next_step - state.start_step;
        let commits = &state.commit_groups[group_index];
        while state.next_commit < commits.len() {
            let commit = commits[state.next_commit].clone();
            if state.ensure_change_ids {
                if let Err(error) = repo.run(&["cherry-pick", "--no-commit", &commit]) {
                    state.pending_cherry_pick =
                        crate::core::draft::check_transient_operation(repo)?.as_deref()
                            == Some("cherry-pick");
                    journal.phase = OperationPhase::Paused;
                    save_state(repo, journal, &state)?;
                    let _ = error;
                    return Err(StaircaseError::OperationPaused {
                        operation_id: journal.operation_id.clone(),
                        kind: journal.kind.clone(),
                    });
                }
                let mut msg = repo.run(&["log", "-1", "--format=%B", &commit])?;
                if !msg.contains("\nChange-Id: I") {
                    msg = format!(
                        "{}\n\nChange-Id: {}",
                        msg.trim_end(),
                        super::utils::generate_change_id()
                    );
                }
                repo.run(&["commit", "--allow-empty", "-C", &commit])?;
                repo.run_with_stdin(&["commit", "--amend", "-F", "-"], &msg)?;
            } else {
                if let Err(error) = repo.run(&["cherry-pick", &commit]) {
                    state.pending_cherry_pick =
                        crate::core::draft::check_transient_operation(repo)?.as_deref()
                            == Some("cherry-pick");
                    journal.phase = OperationPhase::Paused;
                    save_state(repo, journal, &state)?;
                    let _ = error;
                    return Err(StaircaseError::OperationPaused {
                        operation_id: journal.operation_id.clone(),
                        kind: journal.kind.clone(),
                    });
                }
            }
            state.current_base = repo.resolve_commit("HEAD")?;
            state.next_commit += 1;
            save_state(repo, journal, &state)?;
        }
        state.desired.steps[state.next_step].cut = state.current_base.clone();
        state.next_step += 1;
        state.next_commit = 0;
        save_state(repo, journal, &state)?;
    }
    finalize(repo, journal, &state)
}

fn finalize(
    repo: &GitRepo,
    journal: &mut OperationJournal,
    state: &RewriteContinuation,
) -> Result<()> {
    let record_oid =
        if state.old_record_oid.is_some() || !matches!(state.kind.as_str(), "rebase" | "restack") {
            let record = persistence::write_record(
                repo,
                &state.desired,
                &state.user_metadata,
                &state.lifecycle,
                None,
                state.old_record_oid.as_deref(),
                false,
            )?;
            Some(record.record_oid)
        } else {
            None
        };
    let plan = publication_plan(repo, state, record_oid.as_deref(), journal)?;
    publish_resumable(repo, journal, &plan)?;
    let _ = restore_draft_after_success(repo, journal.draft.as_ref())?;
    journal.continuation = None;
    journal.finish(repo)
}

fn read_state(journal: &OperationJournal) -> Result<RewriteContinuation> {
    let value = journal
        .continuation
        .clone()
        .ok_or_else(|| StaircaseError::Other("rewrite journal has no continuation".into()))?;
    let state: RewriteContinuation = serde_json::from_value(value)?;
    if state.schema != "git-staircase/rewrite-continuation" || state.version != 1 {
        return Err(StaircaseError::Other(
            "unsupported rewrite continuation schema".into(),
        ));
    }
    Ok(state)
}

fn save_state(
    repo: &GitRepo,
    journal: &mut OperationJournal,
    state: &RewriteContinuation,
) -> Result<()> {
    let progress_ref = format!("refs/staircase-recovery/{}/progress", journal.operation_id);
    let actual = repo.resolve_ref_opt(&progress_ref)?;
    if actual.as_deref() != Some(state.current_base.as_str()) {
        let command = match actual {
            Some(old) => format!("update {} {} {}", progress_ref, state.current_base, old),
            None => format!("create {} {}", progress_ref, state.current_base),
        };
        repo.update_refs_transaction(&[command])?;
    }
    journal
        .recovery_refs
        .insert("__rewrite_progress__".into(), progress_ref);
    journal.continuation = Some(serde_json::to_value(state)?);
    journal.persist(repo)
}

fn lease_plan(
    repo: &GitRepo,
    old: &StaircaseMetadata,
    new: &StaircaseMetadata,
    old_record_oid: Option<&str>,
    kind: &str,
) -> Result<MutationPlan> {
    let mut plan = MutationPlan::new(kind, Some(old.id.clone()))
        .expected_record(old_record_oid.map(String::from));
    if let Some(oid) = old_record_oid {
        lease(
            repo,
            &mut plan,
            StaircaseRefs::record(&old.id, LifecycleState::Active),
            Some(oid.into()),
        )?;
    }
    let public = StaircaseRefs::public(&old.name);
    if let Some(oid) = old_record_oid {
        if repo.resolve_ref_opt(&public)?.as_deref() == Some(oid) {
            lease(repo, &mut plan, public, Some(oid.into()))?;
        }
    }
    let step_ids: BTreeSet<_> = old
        .steps
        .iter()
        .chain(new.steps.iter())
        .map(|step| step.id.clone())
        .collect();
    for id in step_ids {
        let reference = StaircaseRefs::step(&old.id, &id, LifecycleState::Active);
        let actual = repo.resolve_ref_opt(&reference)?;
        plan.update(reference, actual.clone(), actual);
    }
    let old_owned = owned_branches(old);
    let new_owned = owned_branches(new);
    for reference in old_owned
        .keys()
        .chain(new_owned.keys())
        .collect::<BTreeSet<_>>()
    {
        let actual = repo.resolve_ref_opt(reference)?;
        match old_owned.get(reference.as_str()) {
            Some(expected) if kind != "restack" && actual.as_deref() != Some(expected.as_str()) => {
                return Err(StaircaseError::RefCollision {
                    reference: reference.to_string(),
                    expected: expected.clone(),
                    actual: actual.unwrap_or_else(|| "<missing>".into()),
                });
            }
            None if actual.is_some() => {
                return Err(StaircaseError::RefCollision {
                    reference: reference.to_string(),
                    expected: "<missing>".into(),
                    actual: actual.expect("checked"),
                });
            }
            _ => {}
        }
        plan.update(reference.to_string(), actual.clone(), actual);
    }
    Ok(plan)
}

fn lease(
    repo: &GitRepo,
    plan: &mut MutationPlan,
    reference: String,
    expected: Option<String>,
) -> Result<()> {
    let actual = repo.resolve_ref_opt(&reference)?;
    if actual != expected {
        return Err(StaircaseError::RefCollision {
            reference,
            expected: expected.unwrap_or_else(|| "<missing>".into()),
            actual: actual.unwrap_or_else(|| "<missing>".into()),
        });
    }
    plan.update(reference, actual.clone(), actual);
    Ok(())
}

fn publication_plan(
    repo: &GitRepo,
    state: &RewriteContinuation,
    new_record_oid: Option<&str>,
    journal: &OperationJournal,
) -> Result<MutationPlan> {
    let mut plan = MutationPlan::new(&state.kind, Some(state.desired.id.clone()))
        .expected_record(state.old_record_oid.clone());
    let mut update = |reference: String, planned: Option<String>| -> Result<()> {
        let expected = journal
            .expected_refs
            .get(&reference)
            .cloned()
            .ok_or_else(|| {
                StaircaseError::Other(format!("rewrite did not lease '{}'", reference))
            })?;
        let actual = repo.resolve_ref_opt(&reference)?;
        if actual != expected {
            return Err(StaircaseError::RefCollision {
                reference: reference.clone(),
                expected: expected.unwrap_or_else(|| "<missing>".into()),
                actual: actual.unwrap_or_else(|| "<missing>".into()),
            });
        }
        plan.update(reference, expected, planned);
        Ok(())
    };
    if let Some(oid) = new_record_oid {
        update(
            StaircaseRefs::record(&state.desired.id, LifecycleState::Active),
            Some(oid.into()),
        )?;
        let public = StaircaseRefs::public(&state.original.name);
        if journal.expected_refs.contains_key(&public) {
            update(public, Some(oid.into()))?;
        }

        let old_steps: BTreeMap<_, _> = state
            .original
            .steps
            .iter()
            .map(|step| (step.id.clone(), step))
            .collect();
        let new_steps: BTreeMap<_, _> = state
            .desired
            .steps
            .iter()
            .map(|step| (step.id.clone(), step))
            .collect();
        for id in old_steps
            .keys()
            .chain(new_steps.keys())
            .collect::<BTreeSet<_>>()
        {
            update(
                StaircaseRefs::step(&state.desired.id, id, LifecycleState::Active),
                new_steps.get(id).map(|step| step.cut.clone()),
            )?;
        }
    }
    let old_owned = owned_branches(&state.original);
    let new_owned = owned_branches(&state.desired);
    for reference in old_owned
        .keys()
        .chain(new_owned.keys())
        .collect::<BTreeSet<_>>()
    {
        update(reference.to_string(), new_owned.get(reference).cloned())?;
    }
    Ok(plan)
}

fn owned_branches(metadata: &StaircaseMetadata) -> BTreeMap<String, String> {
    metadata
        .steps
        .iter()
        .filter_map(|step| {
            step.branch.as_ref().map(|branch| {
                (
                    format!("refs/heads/{}", branch.trim_start_matches("refs/heads/")),
                    step.cut.clone(),
                )
            })
        })
        .collect()
}

fn ensure_worktree_safety(
    repo: &GitRepo,
    old: &StaircaseMetadata,
    new: &StaircaseMetadata,
) -> Result<()> {
    let changed: BTreeSet<_> = owned_branches(old)
        .into_iter()
        .chain(owned_branches(new))
        .map(|(reference, _)| reference)
        .collect();
    let current = repo
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone());
    for worktree in repo.worktrees()? {
        let path = worktree
            .path
            .canonicalize()
            .unwrap_or_else(|_| worktree.path.clone());
        if path != current
            && worktree
                .branch
                .as_ref()
                .is_some_and(|branch| changed.contains(branch))
        {
            return Err(StaircaseError::UnsupportedTopology {
                operation: "rewrite".into(),
                reason: format!(
                    "{} is checked out in worktree {}; detach it before rewriting",
                    worktree.branch.expect("checked"),
                    worktree.path.display()
                ),
            });
        }
    }
    Ok(())
}

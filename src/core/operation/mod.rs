pub mod journal;
pub mod lock;
pub mod mutation;
pub mod recovery;

pub use journal::{
    OperationJournal, OperationPhase, OperationResult, abort_active, active_operation,
    ensure_no_active,
};
pub use lock::OperationLocks;
pub use mutation::{MutationPlan, RefMutation};
pub use recovery::{DraftRecovery, restore_draft_after_success};

use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;

pub(crate) fn begin_resumable(
    repo: &GitRepo,
    plan: &MutationPlan,
    continuation: serde_json::Value,
) -> Result<OperationJournal> {
    plan.validate(repo)?;
    ensure_no_active(repo)?;
    let mut journal = OperationJournal::from_plan(repo, plan)?;
    let _locks =
        OperationLocks::acquire(repo, &journal.operation_id, journal.lineage_id.as_deref())?;
    journal.continuation = Some(continuation);
    journal.persist(repo)?;
    journal.create_recovery_refs(repo)?;
    journal.phase = OperationPhase::Publishing;
    journal.persist(repo)?;
    Ok(journal)
}

pub(crate) fn publish_resumable(
    repo: &GitRepo,
    journal: &mut OperationJournal,
    plan: &MutationPlan,
) -> Result<()> {
    for edit in &plan.refs {
        let journal_expected = journal
            .expected_refs
            .get(&edit.reference)
            .cloned()
            .ok_or_else(|| {
                StaircaseError::Other(format!(
                    "resumable operation did not lease '{}'",
                    edit.reference
                ))
            })?;
        if journal_expected != edit.expected {
            return Err(StaircaseError::RefCollision {
                reference: edit.reference.clone(),
                expected: journal_expected.unwrap_or_else(|| "<missing>".into()),
                actual: edit.expected.clone().unwrap_or_else(|| "<missing>".into()),
            });
        }
    }
    plan.validate(repo)?;
    journal.planned_refs = plan
        .refs
        .iter()
        .map(|edit| (edit.reference.clone(), edit.planned.clone()))
        .collect();
    journal.phase = OperationPhase::Publishing;
    journal.persist(repo)?;
    repo.update_refs_transaction(&mutation::ref_commands(
        &journal.expected_refs,
        &journal.planned_refs,
    ))
}

pub fn continue_active(repo: &GitRepo) -> Result<OperationResult> {
    let mut journal = active_operation(repo)?.ok_or(StaircaseError::NoActiveOperation)?;
    let _locks =
        OperationLocks::acquire(repo, &journal.operation_id, journal.lineage_id.as_deref())?;
    if journal.continuation.is_some() {
        return crate::core::rewrite::continue_operation(repo, journal);
    }
    if let Some(operation) = crate::core::draft::check_transient_operation(repo)? {
        let args = match operation.as_str() {
            "rebase" => ["rebase", "--continue"].as_slice(),
            "merge" => ["merge", "--continue"].as_slice(),
            "cherry-pick" => ["cherry-pick", "--continue"].as_slice(),
            "revert" => ["revert", "--continue"].as_slice(),
            _ => {
                return Err(StaircaseError::ExternalOperation {
                    owner: git_operation_owner(&operation).into(),
                    operation,
                });
            }
        };
        repo.command().args(args).env("GIT_EDITOR", "true").run()?;
        if crate::core::draft::check_transient_operation(repo)?.is_some() {
            journal.phase = OperationPhase::Paused;
            journal.persist(repo)?;
            return Ok(OperationResult {
                schema: "git-staircase/operation-result".into(),
                version: 1,
                operation_id: journal.operation_id,
                kind: journal.kind,
                transition: "paused".into(),
                restored_refs: 0,
                draft_restored: false,
            });
        }
    }
    for (reference, planned) in &journal.planned_refs {
        let actual = repo.resolve_ref_opt(reference)?;
        let expected = journal.expected_refs.get(reference).cloned().flatten();
        if actual != *planned && actual != expected {
            return Err(StaircaseError::RefCollision {
                reference: reference.clone(),
                expected: planned.clone().unwrap_or_else(|| "<missing>".into()),
                actual: actual.unwrap_or_else(|| "<missing>".into()),
            });
        }
    }
    let commands = mutation::ref_commands_from_current(repo, &journal.planned_refs)?;
    repo.update_refs_transaction(&commands)?;
    let result = OperationResult {
        schema: "git-staircase/operation-result".into(),
        version: 1,
        operation_id: journal.operation_id.clone(),
        kind: journal.kind.clone(),
        transition: "continued".into(),
        restored_refs: 0,
        draft_restored: false,
    };
    journal.finish(repo)?;
    Ok(result)
}

pub fn external_git_operation(repo: &GitRepo) -> Result<Option<(String, String)>> {
    let active = active_operation(repo)?;
    Ok(
        crate::core::draft::check_transient_operation(repo)?.map(|operation| {
            let owner = if active
                .as_ref()
                .is_some_and(|journal| journal.continuation.is_some())
            {
                "git staircase continue|abort".to_string()
            } else {
                git_operation_owner(&operation).to_string()
            };
            (operation, owner)
        }),
    )
}

fn git_operation_owner(operation: &str) -> &'static str {
    match operation {
        "rebase" => "git rebase --continue|--abort",
        "merge" => "git merge --continue|--abort",
        "cherry-pick" => "git cherry-pick --continue|--abort",
        "revert" => "git revert --continue|--abort",
        "bisect" => "git bisect reset",
        _ => "Git",
    }
}

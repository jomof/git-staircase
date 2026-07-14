use super::journal::{OperationJournal, OperationResult, ensure_no_active_except};
use super::lock::OperationLocks;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
pub struct MutationPlan {
    pub schema: String,
    pub version: u32,
    pub kind: String,
    pub lineage_id: Option<String>,
    pub expected_record_revision: Option<String>,
    pub refs: Vec<RefMutation>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RefMutation {
    pub reference: String,
    pub expected: Option<String>,
    pub planned: Option<String>,
}

impl MutationPlan {
    pub fn new(kind: impl Into<String>, lineage_id: Option<String>) -> Self {
        Self {
            schema: "git-staircase/mutation-plan".into(),
            version: 1,
            kind: kind.into(),
            lineage_id,
            expected_record_revision: None,
            refs: Vec::new(),
        }
    }

    pub fn expected_record(mut self, oid: Option<String>) -> Self {
        self.expected_record_revision = oid;
        self
    }

    pub fn update(
        &mut self,
        reference: impl Into<String>,
        expected: Option<String>,
        planned: Option<String>,
    ) {
        self.refs.push(RefMutation {
            reference: reference.into(),
            expected,
            planned,
        });
    }

    pub fn validate(&self, repo: &GitRepo) -> Result<()> {
        let mut seen = BTreeSet::new();
        for edit in &self.refs {
            if !seen.insert(&edit.reference) {
                return Err(StaircaseError::Other(format!(
                    "mutation plan contains duplicate ref '{}'",
                    edit.reference
                )));
            }
            let actual = repo.resolve_ref_opt(&edit.reference)?;
            if actual != edit.expected {
                return Err(StaircaseError::RefCollision {
                    reference: edit.reference.clone(),
                    expected: edit.expected.clone().unwrap_or_else(|| "<missing>".into()),
                    actual: actual.unwrap_or_else(|| "<missing>".into()),
                });
            }
        }
        Ok(())
    }

    pub fn publish(&self, repo: &GitRepo, dry_run: bool) -> Result<Option<OperationResult>> {
        self.validate(repo)?;
        if dry_run {
            return Ok(None);
        }
        let mut journal = OperationJournal::from_plan(repo, self)?;
        let _locks =
            OperationLocks::acquire(repo, &journal.operation_id, journal.lineage_id.as_deref())?;
        ensure_no_active_except(repo, None)?;
        journal.publish(repo)?;
        let result = OperationResult {
            schema: "git-staircase/operation-result".into(),
            version: 1,
            operation_id: journal.operation_id.clone(),
            kind: journal.kind.clone(),
            transition: "completed".into(),
            restored_refs: 0,
            draft_restored: false,
        };
        journal.finish(repo)?;
        Ok(Some(result))
    }
}

pub(crate) fn ref_commands(
    expected: &BTreeMap<String, Option<String>>,
    planned: &BTreeMap<String, Option<String>>,
) -> Vec<String> {
    let mut commands = Vec::new();
    for (reference, new) in planned {
        let old = expected.get(reference).and_then(Option::as_deref);
        push_ref_command(&mut commands, reference, old, new.as_deref());
    }
    commands
}

pub(crate) fn ref_commands_from_current(
    repo: &GitRepo,
    planned: &BTreeMap<String, Option<String>>,
) -> Result<Vec<String>> {
    let mut commands = Vec::new();
    for (reference, new) in planned {
        let old = repo.resolve_ref_opt(reference)?;
        if old != *new {
            push_ref_command(&mut commands, reference, old.as_deref(), new.as_deref());
        }
    }
    Ok(commands)
}

pub(crate) fn push_ref_command(
    commands: &mut Vec<String>,
    reference: &str,
    old: Option<&str>,
    new: Option<&str>,
) {
    match (old, new) {
        (None, Some(new)) => commands.push(format!("create {} {}", reference, new)),
        (Some(old), Some(new)) => commands.push(format!("update {} {} {}", reference, new, old)),
        (Some(old), None) => commands.push(format!("delete {} {}", reference, old)),
        (None, None) => {}
    }
}

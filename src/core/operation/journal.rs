use super::mutation::MutationPlan;
use super::recovery::{DraftRecovery, capture_draft, restore_draft};
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const JOURNAL_SCHEMA: &str = "git-staircase/operation-journal";
pub const JOURNAL_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OperationPhase {
    Planned,
    Publishing,
    Paused,
    Aborting,
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationJournal {
    pub schema: String,
    pub version: u32,
    pub operation_id: String,
    pub kind: String,
    pub phase: OperationPhase,
    pub repository_identity: String,
    pub lineage_id: Option<String>,
    pub expected_record_revision: Option<String>,
    pub expected_refs: BTreeMap<String, Option<String>>,
    pub planned_refs: BTreeMap<String, Option<String>>,
    pub recovery_refs: BTreeMap<String, String>,
    pub draft: Option<DraftRecovery>,
    pub disposition: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperationResult {
    pub schema: String,
    pub version: u32,
    pub operation_id: String,
    pub kind: String,
    pub transition: String,
    pub restored_refs: usize,
    pub draft_restored: bool,
}

impl OperationJournal {
    pub fn from_plan(repo: &GitRepo, plan: &MutationPlan) -> Result<Self> {
        let expected_refs = plan
            .refs
            .iter()
            .map(|edit| (edit.reference.clone(), edit.expected.clone()))
            .collect();
        let planned_refs = plan
            .refs
            .iter()
            .map(|edit| (edit.reference.clone(), edit.planned.clone()))
            .collect();
        Ok(Self {
            schema: JOURNAL_SCHEMA.into(),
            version: JOURNAL_VERSION,
            operation_id: Uuid::new_v4().to_string(),
            kind: plan.kind.clone(),
            phase: OperationPhase::Planned,
            repository_identity: repo.repository_identity()?,
            lineage_id: plan.lineage_id.clone(),
            expected_record_revision: plan.expected_record_revision.clone(),
            expected_refs,
            planned_refs,
            recovery_refs: BTreeMap::new(),
            draft: capture_draft(repo)?,
            disposition: "continue-or-abort".into(),
            continuation: None,
        })
    }

    pub fn publish(&mut self, repo: &GitRepo) -> Result<()> {
        self.persist(repo)?;
        self.create_recovery_refs(repo)?;
        self.persist(repo)?;
        self.phase = OperationPhase::Publishing;
        self.persist(repo)?;
        let commands = super::mutation::ref_commands(&self.expected_refs, &self.planned_refs);
        if let Err(error) = repo.update_refs_transaction(&commands) {
            self.phase = OperationPhase::Paused;
            self.persist(repo)?;
            return Err(error);
        }
        Ok(())
    }

    pub fn create_recovery_refs(&mut self, repo: &GitRepo) -> Result<()> {
        let mut commands = Vec::new();
        for (reference, oid) in &self.expected_refs {
            let Some(oid) = oid else { continue };
            let recovery = recovery_ref(&self.operation_id, reference);
            if let Some(actual) = repo.resolve_ref_opt(&recovery)? {
                commands.push(format!("update {} {} {}", recovery, oid, actual));
            } else {
                commands.push(format!("create {} {}", recovery, oid));
            }
            self.recovery_refs.insert(reference.clone(), recovery);
        }
        repo.update_refs_transaction(&commands)
    }

    pub fn persist(&self, repo: &GitRepo) -> Result<()> {
        validate_journal(self)?;
        let path = journal_path(repo, &self.operation_id)?;
        atomic_json(&path, self)
    }

    pub fn finish(&mut self, repo: &GitRepo) -> Result<()> {
        self.phase = OperationPhase::Completed;
        self.persist(repo)?;
        remove_recovery_refs(repo, self)?;
        let path = journal_path(repo, &self.operation_id)?;
        fs::remove_file(path)?;
        sync_parent(&journals_dir(repo)?)?;
        Ok(())
    }
}

pub fn active_operation(repo: &GitRepo) -> Result<Option<OperationJournal>> {
    let dir = journals_dir(repo)?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    paths.sort();
    for path in paths {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(journal) = serde_json::from_str::<OperationJournal>(&content) else {
            continue;
        };
        if validate_journal(&journal).is_ok() {
            return Ok(Some(journal));
        }
    }
    Ok(None)
}

pub fn ensure_no_active(repo: &GitRepo) -> Result<()> {
    ensure_no_active_except(repo, None)
}

pub fn ensure_no_active_except(repo: &GitRepo, allowed_id: Option<&str>) -> Result<()> {
    if let Some(active) = active_operation(repo)? {
        if allowed_id != Some(active.operation_id.as_str()) {
            return Err(StaircaseError::OperationInProgress {
                operation_id: active.operation_id,
                kind: active.kind,
            });
        }
    }
    Ok(())
}

fn validate_journal(journal: &OperationJournal) -> Result<()> {
    if journal.schema != JOURNAL_SCHEMA || journal.version != JOURNAL_VERSION {
        return Err(StaircaseError::Other(
            "unsupported operation journal schema".into(),
        ));
    }
    Ok(())
}

fn journal_path(repo: &GitRepo, operation_id: &str) -> Result<PathBuf> {
    Ok(journals_dir(repo)?.join(format!("{}.json", operation_id)))
}

fn journals_dir(repo: &GitRepo) -> Result<PathBuf> {
    let path = staircase_dir(repo)?.join("journals");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn staircase_dir(repo: &GitRepo) -> Result<PathBuf> {
    let path = repo.common_dir()?.join("staircase");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| StaircaseError::Other("journal path has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    sync_parent(parent)
}

fn sync_parent(parent: &Path) -> Result<()> {
    File::open(parent)?.sync_all()?;
    Ok(())
}

const RECOVERY_PREFIX: &str = "refs/staircase-recovery/";

fn recovery_ref(operation_id: &str, reference: &str) -> String {
    if reference.ends_with("/record") {
        return format!("{}{}/record", RECOVERY_PREFIX, operation_id);
    }
    if let Some((_, step_id)) = reference.split_once("/steps/") {
        return format!("{}{}/steps/{}", RECOVERY_PREFIX, operation_id, step_id);
    }
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(reference.as_bytes());
    format!("{}{}/owned/{:x}", RECOVERY_PREFIX, operation_id, digest)
}

fn remove_recovery_refs(repo: &GitRepo, journal: &OperationJournal) -> Result<()> {
    let mut commands = Vec::new();
    for recovery in journal.recovery_refs.values() {
        if let Some(actual) = repo.resolve_ref_opt(recovery)? {
            commands.push(format!("delete {} {}", recovery, actual));
        }
    }
    repo.update_refs_transaction(&commands)
}

pub fn abort_active(repo: &GitRepo) -> Result<OperationResult> {
    let mut journal = match active_operation(repo)? {
        Some(journal) => journal,
        None => {
            let dir = journals_dir(repo)?;
            let mut cleaned = false;
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|v| v.to_str()) == Some("json") {
                    fs::remove_file(path)?;
                    cleaned = true;
                }
            }
            if cleaned {
                return Ok(OperationResult {
                    schema: "git-staircase/operation-result".into(),
                    version: 1,
                    operation_id: "none".into(),
                    kind: "none".into(),
                    transition: "aborted".into(),
                    restored_refs: 0,
                    draft_restored: false,
                });
            }
            return Err(StaircaseError::NoActiveOperation);
        }
    };
    let _locks = super::lock::OperationLocks::acquire(
        repo,
        &journal.operation_id,
        journal.lineage_id.as_deref(),
    )?;
    journal.phase = OperationPhase::Aborting;
    journal.persist(repo)?;
    if let Some(operation) = crate::core::draft::check_transient_operation(repo)? {
        let args = match operation.as_str() {
            "rebase" => Some(["rebase", "--abort"].as_slice()),
            "merge" => Some(["merge", "--abort"].as_slice()),
            "cherry-pick" => Some(["cherry-pick", "--abort"].as_slice()),
            "revert" => Some(["revert", "--abort"].as_slice()),
            "bisect" => Some(["bisect", "reset"].as_slice()),
            _ => None,
        };
        if let Some(args) = args {
            repo.command().args(args).run()?;
        }
    }

    let mut commands = Vec::new();
    let mut restored = 0;
    for (reference, expected) in &journal.expected_refs {
        let current = repo.resolve_ref_opt(reference)?;
        let planned = journal.planned_refs.get(reference).cloned().flatten();
        if current == *expected {
            continue;
        }
        if current != planned {
            return Err(StaircaseError::RefCollision {
                reference: reference.clone(),
                expected: planned.unwrap_or_else(|| "<missing>".into()),
                actual: current.unwrap_or_else(|| "<missing>".into()),
            });
        }
        super::mutation::push_ref_command(
            &mut commands,
            reference,
            current.as_deref(),
            expected.as_deref(),
        );
        restored += 1;
    }
    repo.update_refs_transaction(&commands)?;
    let draft_restored = restore_draft(repo, journal.draft.as_ref())?;
    let result = OperationResult {
        schema: "git-staircase/operation-result".into(),
        version: 1,
        operation_id: journal.operation_id.clone(),
        kind: journal.kind.clone(),
        transition: "aborted".into(),
        restored_refs: restored,
        draft_restored,
    };
    journal.finish(repo)?;
    Ok(result)
}

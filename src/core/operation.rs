use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const JOURNAL_SCHEMA: &str = "git-staircase/operation-journal";
const JOURNAL_VERSION: u32 = 1;
const RECOVERY_PREFIX: &str = "refs/staircase-recovery/";

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
pub struct DraftRecovery {
    pub head_oid: String,
    pub head_ref: Option<String>,
    pub index_tree_oid: Option<String>,
    pub index_snapshot: Option<String>,
    #[serde(default)]
    pub dirty_files: Vec<crate::model::DraftFileSnapshot>,
    pub unstaged_patch: String,
    pub untracked_paths: Vec<String>,
    #[serde(default)]
    pub untracked_files: Vec<crate::model::DraftFileSnapshot>,
    pub attachment_json: Option<String>,
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

impl OperationJournal {
    pub(crate) fn from_plan(repo: &GitRepo, plan: &MutationPlan) -> Result<Self> {
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

    fn publish(&mut self, repo: &GitRepo) -> Result<()> {
        self.persist(repo)?;
        self.create_recovery_refs(repo)?;
        self.persist(repo)?;
        self.phase = OperationPhase::Publishing;
        self.persist(repo)?;
        let commands = ref_commands(&self.expected_refs, &self.planned_refs);
        if let Err(error) = repo.update_refs_transaction(&commands) {
            self.phase = OperationPhase::Paused;
            self.persist(repo)?;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn create_recovery_refs(&mut self, repo: &GitRepo) -> Result<()> {
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

    pub(crate) fn persist(&self, repo: &GitRepo) -> Result<()> {
        validate_journal(self)?;
        let path = journal_path(repo, &self.operation_id)?;
        atomic_json(&path, self)
    }

    pub(crate) fn finish(&mut self, repo: &GitRepo) -> Result<()> {
        self.phase = OperationPhase::Completed;
        self.persist(repo)?;
        remove_recovery_refs(repo, self)?;
        let path = journal_path(repo, &self.operation_id)?;
        fs::remove_file(path)?;
        sync_parent(&journals_dir(repo)?)?;
        Ok(())
    }
}

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
    repo.update_refs_transaction(&ref_commands(&journal.expected_refs, &journal.planned_refs))
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
    let Some(path) = paths.first() else {
        return Ok(None);
    };
    let journal: OperationJournal = serde_json::from_str(&fs::read_to_string(path)?)?;
    validate_journal(&journal)?;
    Ok(Some(journal))
}

pub fn ensure_no_active(repo: &GitRepo) -> Result<()> {
    ensure_no_active_except(repo, None)
}

fn ensure_no_active_except(repo: &GitRepo, allowed_id: Option<&str>) -> Result<()> {
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
    let commands = ref_commands_from_current(repo, &journal.planned_refs)?;
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

pub fn abort_active(repo: &GitRepo) -> Result<OperationResult> {
    let mut journal = active_operation(repo)?.ok_or(StaircaseError::NoActiveOperation)?;
    let _locks =
        OperationLocks::acquire(repo, &journal.operation_id, journal.lineage_id.as_deref())?;
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
        push_ref_command(
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

fn validate_journal(journal: &OperationJournal) -> Result<()> {
    if journal.schema != JOURNAL_SCHEMA || journal.version != JOURNAL_VERSION {
        return Err(StaircaseError::Other(
            "unsupported operation journal schema".into(),
        ));
    }
    Ok(())
}

fn capture_draft(repo: &GitRepo) -> Result<Option<DraftRecovery>> {
    let head_oid = match repo.resolve_commit_opt("HEAD")? {
        Some(oid) => oid,
        None => return Ok(None),
    };
    let (index_tree_oid, index_snapshot, dirty_files) = match repo.run(&["write-tree"]) {
        Ok(oid) => (Some(oid.trim().to_string()), None, Vec::new()),
        Err(_) => {
            let (snapshot, files) = capture_conflicted_state(repo)?;
            (None, snapshot, files)
        }
    };
    let unstaged_patch = repo
        .command()
        .args(["diff", "--binary", "--no-ext-diff"])
        .trim(false)
        .run()
        .unwrap_or_default();
    let untracked = repo
        .command()
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .trim(false)
        .run()
        .unwrap_or_default();
    let attachment_path = repo.git_dir()?.join("staircase-draft.json");
    let attachment_json = fs::read_to_string(attachment_path).ok();
    Ok(Some(DraftRecovery {
        head_oid,
        head_ref: repo.current_branch()?,
        index_tree_oid,
        index_snapshot,
        dirty_files,
        unstaged_patch,
        untracked_paths: untracked
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect(),
        untracked_files: crate::core::draft::capture_untracked_files(repo)?,
        attachment_json,
    }))
}

pub(crate) fn restore_draft(repo: &GitRepo, draft: Option<&DraftRecovery>) -> Result<bool> {
    let Some(draft) = draft else { return Ok(false) };
    if let Some(head_ref) = &draft.head_ref {
        let branch = head_ref.strip_prefix("refs/heads/").unwrap_or(head_ref);
        repo.run(&["checkout", "-f", branch])?;
    } else {
        repo.run(&["checkout", "--detach", "-f", &draft.head_oid])?;
    }
    if let Some(tree) = &draft.index_tree_oid {
        repo.run(&["read-tree", tree])?;
        repo.run(&["checkout-index", "-a", "-f"])?;
    } else if let Some(snapshot) = &draft.index_snapshot {
        repo.run(&["read-tree", "--empty"])?;
        repo.run_with_stdin(&["update-index", "--index-info"], snapshot)?;
    }
    if !draft.dirty_files.is_empty() {
        restore_dirty_files(repo, &draft.dirty_files)?;
    }
    if !draft.unstaged_patch.is_empty() {
        let _ = repo.run_with_stdin(&["apply", "--binary"], &draft.unstaged_patch);
    }
    crate::core::draft::restore_untracked_files(repo, &draft.untracked_files)?;
    let attachment_path = repo.git_dir()?.join("staircase-draft.json");
    match &draft.attachment_json {
        Some(content) => fs::write(attachment_path, content)?,
        None if attachment_path.exists() => fs::remove_file(attachment_path)?,
        None => {}
    }
    Ok(true)
}

pub(crate) fn restore_draft_after_success(
    repo: &GitRepo,
    draft: Option<&DraftRecovery>,
) -> Result<bool> {
    let Some(draft) = draft else { return Ok(false) };
    let clean_index = draft.index_tree_oid.as_ref().is_some_and(|tree| {
        repo.get_tree_id(&draft.head_oid).ok().as_deref() == Some(tree.as_str())
    });
    if clean_index
        && draft.unstaged_patch.is_empty()
        && draft.untracked_files.is_empty()
        && draft.attachment_json.is_none()
    {
        if let Some(head_ref) = &draft.head_ref {
            let branch = head_ref.strip_prefix("refs/heads/").unwrap_or(head_ref);
            repo.run(&["checkout", "-f", branch])?;
        } else {
            repo.run(&["checkout", "--detach", "-f", &draft.head_oid])?;
        }
        return Ok(true);
    }
    restore_draft(repo, Some(draft))
}

fn ref_commands(
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

fn ref_commands_from_current(
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

fn push_ref_command(
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

fn recovery_ref(operation_id: &str, reference: &str) -> String {
    if reference.ends_with("/record") {
        return format!("{}{}/record", RECOVERY_PREFIX, operation_id);
    }
    if let Some((_, step_id)) = reference.split_once("/steps/") {
        return format!("{}{}/steps/{}", RECOVERY_PREFIX, operation_id, step_id);
    }
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

fn common_dir(repo: &GitRepo) -> Result<PathBuf> {
    repo.common_dir()
}

fn staircase_dir(repo: &GitRepo) -> Result<PathBuf> {
    let path = common_dir(repo)?.join("staircase");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn journals_dir(repo: &GitRepo) -> Result<PathBuf> {
    let path = staircase_dir(repo)?.join("journals");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn locks_dir(repo: &GitRepo) -> Result<PathBuf> {
    let path = staircase_dir(repo)?.join("locks");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn journal_path(repo: &GitRepo, operation_id: &str) -> Result<PathBuf> {
    Ok(journals_dir(repo)?.join(format!("{}.json", operation_id)))
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

struct RepositoryLock {
    path: PathBuf,
}

impl RepositoryLock {
    fn acquire(repo: &GitRepo, operation_id: &str, name: &str) -> Result<Self> {
        let path = locks_dir(repo)?.join(name);
        let open = || OpenOptions::new().write(true).create_new(true).open(&path);
        let mut file = match open() {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = fs::read_to_string(&path)
                    .ok()
                    .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok());
                let pid = existing
                    .as_ref()
                    .and_then(|lock| lock.get("pid"))
                    .and_then(|pid| pid.as_u64());
                let live = pid
                    .map(|pid| {
                        std::process::Command::new("kill")
                            .args(["-0", &pid.to_string()])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status()
                            .is_ok_and(|status| status.success())
                    })
                    .unwrap_or(true);
                if live {
                    return Err(StaircaseError::OperationInProgress {
                        operation_id: existing
                            .as_ref()
                            .and_then(|lock| lock.get("operation_id"))
                            .and_then(|id| id.as_str())
                            .unwrap_or(operation_id)
                            .into(),
                        kind: "repository-lock".into(),
                    });
                }
                fs::remove_file(&path)?;
                open()?
            }
            Err(error) => return Err(error.into()),
        };
        let lock = serde_json::json!({
            "schema": "git-staircase/operation-lock",
            "version": 1,
            "pid": std::process::id(),
            "operation_id": operation_id,
            "nonce": Uuid::new_v4().to_string(),
        });
        file.write_all(serde_json::to_string(&lock)?.as_bytes())?;
        file.sync_all()?;
        Ok(Self { path })
    }
}

struct OperationLocks {
    _repository: RepositoryLock,
    _lineage: Option<RepositoryLock>,
}

impl OperationLocks {
    fn acquire(repo: &GitRepo, operation_id: &str, lineage_id: Option<&str>) -> Result<Self> {
        let repository = RepositoryLock::acquire(repo, operation_id, "repository.lock")?;
        let lineage = lineage_id
            .map(|lineage_id| {
                RepositoryLock::acquire(repo, operation_id, &format!("lineage-{}.lock", lineage_id))
            })
            .transpose()?;
        Ok(Self {
            _repository: repository,
            _lineage: lineage,
        })
    }
}

impl Drop for RepositoryLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn capture_conflicted_state(
    repo: &GitRepo,
) -> Result<(Option<String>, Vec<crate::model::DraftFileSnapshot>)> {
    let index_snapshot = repo.run(&["ls-files", "-s"]).ok();
    let mut paths = std::collections::BTreeSet::new();
    let modified = repo
        .command()
        .args(["ls-files", "-m", "-z"])
        .trim(false)
        .run()
        .unwrap_or_default();
    for path in modified.split('\0').filter(|p| !p.is_empty()) {
        paths.insert(path.to_string());
    }
    let unmerged = repo
        .command()
        .args(["ls-files", "-u", "-z"])
        .trim(false)
        .run()
        .unwrap_or_default();
    for line in unmerged.split('\0').filter(|p| !p.is_empty()) {
        if let Some(path) = line.split('\t').last() {
            paths.insert(path.to_string());
        }
    }
    let mut files = Vec::new();
    for path in paths {
        let absolute = repo.workdir.join(&path);
        if !absolute.exists() {
            continue;
        }
        let metadata = fs::symlink_metadata(&absolute)?;
        let (kind, content) = if metadata.file_type().is_symlink() {
            (
                "symlink".to_string(),
                fs::read_link(&absolute)?.into_os_string().into_vec(),
            )
        } else if metadata.is_file() {
            ("regular".to_string(), fs::read(&absolute)?)
        } else {
            continue;
        };
        files.push(crate::model::DraftFileSnapshot {
            path: path.to_string(),
            kind,
            mode: metadata.permissions().mode(),
            content_hex: crate::core::draft::hex_encode(&content),
        });
    }
    Ok((index_snapshot, files))
}

fn restore_dirty_files(repo: &GitRepo, files: &[crate::model::DraftFileSnapshot]) -> Result<()> {
    for file in files {
        let absolute = repo.workdir.join(&file.path);
        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = crate::core::draft::hex_decode(&file.content_hex)?;
        match file.kind.as_str() {
            "regular" => {
                fs::write(&absolute, content)?;
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&absolute, fs::Permissions::from_mode(file.mode))?;
            }
            "symlink" => {
                let _ = fs::remove_file(&absolute);
                use std::os::unix::ffi::OsStringExt;
                std::os::unix::fs::symlink(std::ffi::OsString::from_vec(content), &absolute)?;
            }
            _ => {}
        }
    }
    Ok(())
}

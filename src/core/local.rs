use crate::core::operation::MutationPlan;
use crate::core::persistence;
use crate::core::refs::StaircaseRefs;
use crate::core::resolved::{ResolvedSelector, adopt};
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{
    LifecycleState, StaircaseMetadata, StaircaseRecord, StaircaseUserMetadata, Step,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryOverride {
    pub id: String,
    pub kind: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalMutationResult {
    pub schema: String,
    pub version: u32,
    pub kind: String,
    pub staircase_id: String,
    pub staircase_name: String,
    pub record_oid: Option<String>,
    pub dry_run: bool,
    pub changed_refs: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutState {
    pub schema: String,
    pub version: u32,
    pub staircase_id: String,
    pub profile: Option<String>,
    pub base: Option<String>,
    pub state: String,
    pub branches: Vec<LayoutBranch>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutBranch {
    pub step_id: String,
    pub step_name: String,
    pub expected_ref: Option<String>,
    pub expected_oid: String,
    pub actual_oid: Option<String>,
}

pub fn append(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    revision_range: &str,
    new_step: bool,
    branch: Option<&str>,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    let mut metadata = selector.metadata().clone();
    ensure_active(&metadata)?;
    let old_top = metadata
        .steps
        .last()
        .ok_or_else(|| StaircaseError::InvalidStructure("empty staircase".into()))?
        .cut
        .clone();
    let commits = resolve_linear_append(repo, &old_top, revision_range)?;
    let new_top = commits
        .last()
        .expect("resolve_linear_append rejects empty ranges")
        .clone();

    if new_step {
        let branch = branch.ok_or_else(|| {
            StaircaseError::Other("--new-step requires --branch <local-branch>".into())
        })?;
        validate_local_branch(repo, branch)?;
        metadata.steps.push(Step {
            id: Uuid::new_v4().to_string(),
            name: branch.to_string(),
            cut: new_top,
            branch: Some(branch.to_string()),
        });
    } else {
        if branch.is_some() {
            return Err(StaircaseError::Other(
                "--branch is valid only with --new-step".into(),
            ));
        }
        metadata.steps.last_mut().expect("top was checked").cut = new_top;
    }

    if selector.is_managed() {
        publish_metadata(repo, selector, metadata, "append", dry_run)
    } else {
        let mut refs = Vec::new();
        let tip = metadata.steps.last().expect("nonempty");
        let branch = tip.branch.as_ref().ok_or_else(|| {
            StaircaseError::Other("append without a materializing branch requires adoption".into())
        })?;
        let reference = local_ref(branch);
        let expected = repo.resolve_ref_opt(&reference)?;
        if !new_step && expected.as_deref() != Some(old_top.as_str()) {
            return Err(StaircaseError::RefCollision {
                reference,
                expected: old_top,
                actual: expected.unwrap_or_else(|| "<missing>".into()),
            });
        }
        let mut plan = MutationPlan::new("append", None);
        plan.update(reference.clone(), expected, Some(tip.cut.clone()));
        plan.publish(repo, dry_run)?;
        refs.push(reference);
        Ok(result("append", &metadata, None, dry_run, refs))
    }
}

pub fn normalize(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    if !selector.is_managed() {
        return Err(StaircaseError::Other(
            "normalization requiring retained state requires a managed staircase".into(),
        ));
    }
    let metadata = selector.metadata().clone();
    publish_metadata(repo, selector, metadata, "normalize", dry_run)
}

pub fn layout_state(repo: &GitRepo, selector: &ResolvedSelector) -> Result<LayoutState> {
    let metadata = selector.metadata();
    let expected_names = expected_layout_names(metadata)?;
    let mut branches = Vec::new();
    let mut clean = true;
    for (index, step) in metadata.steps.iter().enumerate() {
        let expected_ref = expected_names
            .as_ref()
            .map(|names| local_ref(&names[index]));
        let actual_oid = expected_ref
            .as_ref()
            .map(|reference| repo.resolve_ref_opt(reference))
            .transpose()?
            .flatten();
        if expected_ref.is_some() && actual_oid.as_deref() != Some(step.cut.as_str()) {
            clean = false;
        }
        branches.push(LayoutBranch {
            step_id: step.id.clone(),
            step_name: step.name.clone(),
            expected_ref,
            expected_oid: step.cut.clone(),
            actual_oid,
        });
    }
    Ok(LayoutState {
        schema: "git-staircase/layout-state".into(),
        version: 1,
        staircase_id: metadata.id.clone(),
        profile: metadata.primary_branch_layout.clone(),
        base: metadata.branch_layout_base.clone(),
        state: if metadata.primary_branch_layout.is_none() || clean {
            "clean"
        } else {
            "dirty"
        }
        .into(),
        branches,
    })
}

pub fn set_layout(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    base: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    validate_local_branch(repo, base)?;
    let mut metadata = require_managed(selector)?.clone();
    metadata.primary_branch_layout = Some("sequential-v1".into());
    metadata.branch_layout_base = Some(base.into());
    apply_layout_names(&mut metadata)?;
    publish_metadata(repo, selector, metadata, "layout-set", dry_run)
}

pub fn unset_layout(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    let mut metadata = require_managed(selector)?.clone();
    metadata.primary_branch_layout = None;
    metadata.branch_layout_base = None;
    publish_metadata(repo, selector, metadata, "layout-unset", dry_run)
}

pub fn assign_step_branch(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    step_index: usize,
    name: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    validate_local_branch(repo, name)?;
    let mut metadata = require_managed(selector)?.clone();
    let step = metadata.steps.get_mut(step_index).ok_or_else(|| {
        StaircaseError::InvalidStructure(format!("step {} is out of range", step_index + 1))
    })?;
    step.branch = Some(name.into());
    metadata.primary_branch_layout = None;
    metadata.branch_layout_base = None;
    publish_metadata(repo, selector, metadata, "layout-branch", dry_run)
}

pub fn policy_values(
    repo: &GitRepo,
    selector: &ResolvedSelector,
) -> Result<BTreeMap<String, Value>> {
    let record = current_record(repo, selector)?;
    Ok(extension_map(&record.user_metadata, "policies"))
}

pub fn update_policies(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    assignments: &[(String, Option<Value>)],
    dry_run: bool,
) -> Result<LocalMutationResult> {
    require_managed(selector)?;
    let mut record = current_record(repo, selector)?;
    let mut policies = extension_map(&record.user_metadata, "policies");
    for (key, value) in assignments {
        validate_policy_key(key)?;
        match value {
            Some(value) => {
                policies.insert(key.clone(), value.clone());
            }
            None => {
                policies.remove(key);
            }
        }
    }
    record.user_metadata.extensions.insert(
        "git-staircase.policies".into(),
        serde_json::to_value(&policies)?,
    );
    publish_record_parts(
        repo,
        selector,
        record.metadata,
        record.user_metadata,
        "policy",
        dry_run,
    )
}

pub fn discovery_overrides(
    repo: &GitRepo,
    selector: &ResolvedSelector,
) -> Result<Vec<DiscoveryOverride>> {
    let record = current_record(repo, selector)?;
    let value = record
        .user_metadata
        .extensions
        .get("git-staircase.discovery-overrides")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    Ok(serde_json::from_value(value)?)
}

pub fn add_discovery_override(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    kind: &str,
    raw_value: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    require_managed(selector)?;
    let value = canonical_override_value(repo, kind, raw_value)?;
    let mut record = current_record(repo, selector)?;
    let mut overrides = discovery_overrides(repo, selector)?;
    if !overrides
        .iter()
        .any(|item| item.kind == kind && item.value == value)
    {
        overrides.push(DiscoveryOverride {
            id: Uuid::new_v4().to_string(),
            kind: kind.into(),
            value,
        });
    }
    record.user_metadata.extensions.insert(
        "git-staircase.discovery-overrides".into(),
        serde_json::to_value(overrides)?,
    );
    publish_record_parts(
        repo,
        selector,
        record.metadata,
        record.user_metadata,
        "discovery-override",
        dry_run,
    )
}

pub fn clear_discovery_override(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    id: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    require_managed(selector)?;
    let mut record = current_record(repo, selector)?;
    let mut overrides = discovery_overrides(repo, selector)?;
    let original_len = overrides.len();
    overrides.retain(|item| item.id != id);
    if overrides.len() == original_len {
        return Err(StaircaseError::NotFound(id.into()));
    }
    record.user_metadata.extensions.insert(
        "git-staircase.discovery-overrides".into(),
        serde_json::to_value(overrides)?,
    );
    publish_record_parts(
        repo,
        selector,
        record.metadata,
        record.user_metadata,
        "discovery-clear",
        dry_run,
    )
}

pub fn name_staircase(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    new_name: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    validate_staircase_name(repo, new_name)?;
    check_name_available(repo, new_name, None)?;
    if !selector.is_managed() {
        if dry_run {
            return Ok(result(
                "name",
                selector.metadata(),
                None,
                true,
                vec![StaircaseRefs::public(new_name)],
            ));
        }
        let mut metadata = selector.metadata().clone();
        metadata.name = new_name.into();
        let adopted = adopt(repo, &metadata)?;
        let record = current_record_for_metadata(repo, &adopted)?;
        return Ok(result(
            "name",
            &adopted,
            Some(record.record_oid),
            false,
            vec![StaircaseRefs::public(new_name)],
        ));
    }
    let record = current_record(repo, selector)?;
    let reference = StaircaseRefs::public(new_name);
    let mut plan = MutationPlan::new("name", Some(record.metadata.id.clone()))
        .expected_record(Some(record.record_oid.clone()));
    plan.update(reference.clone(), None, Some(record.record_oid.clone()));
    plan.publish(repo, dry_run)?;
    Ok(result(
        "name",
        &record.metadata,
        Some(record.record_oid),
        dry_run,
        vec![reference],
    ))
}

pub fn rename_staircase(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    new_name: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    validate_staircase_name(repo, new_name)?;
    let record = current_record(repo, selector)?;
    check_name_available(repo, new_name, Some(&record.metadata.id))?;
    let old_ref = StaircaseRefs::public(&selector.metadata().name);
    let new_ref = StaircaseRefs::public(new_name);
    let old_oid = repo.resolve_ref_opt(&old_ref)?;
    if old_oid.as_deref() != Some(record.record_oid.as_str()) {
        return Err(StaircaseError::ConcurrentRecordUpdate {
            reference: old_ref,
            expected: record.record_oid,
            actual: old_oid.unwrap_or_else(|| "<missing>".into()),
        });
    }
    let mut plan = MutationPlan::new("rename", Some(record.metadata.id.clone()))
        .expected_record(Some(record.record_oid.clone()));
    plan.update(old_ref.clone(), old_oid, None);
    plan.update(new_ref.clone(), None, Some(record.record_oid.clone()));
    plan.publish(repo, dry_run)?;
    Ok(result(
        "rename",
        &record.metadata,
        Some(record.record_oid),
        dry_run,
        vec![old_ref, new_ref],
    ))
}

pub fn unname_staircase(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    let record = current_record(repo, selector)?;
    let reference = StaircaseRefs::public(&selector.metadata().name);
    let current = repo.resolve_ref_opt(&reference)?;
    if current.as_deref() != Some(record.record_oid.as_str()) {
        return Err(StaircaseError::ConcurrentRecordUpdate {
            reference,
            expected: record.record_oid,
            actual: current.unwrap_or_else(|| "<missing>".into()),
        });
    }
    let mut plan = MutationPlan::new("unname", Some(record.metadata.id.clone()))
        .expected_record(Some(record.record_oid.clone()));
    plan.update(reference.clone(), current, None);
    plan.publish(repo, dry_run)?;
    Ok(result(
        "unname",
        &record.metadata,
        Some(record.record_oid),
        dry_run,
        vec![reference],
    ))
}

pub fn publish_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    metadata: StaircaseMetadata,
    kind: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    let old = current_record(repo, selector)?;
    publish_record_parts(repo, selector, metadata, old.user_metadata, kind, dry_run)
}

fn publish_record_parts(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    metadata: StaircaseMetadata,
    user_metadata: StaircaseUserMetadata,
    kind: &str,
    dry_run: bool,
) -> Result<LocalMutationResult> {
    publish_record_parts_extra(repo, selector, metadata, user_metadata, kind, dry_run, &[])
}

pub(crate) fn publish_record_parts_extra(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    mut metadata: StaircaseMetadata,
    user_metadata: StaircaseUserMetadata,
    kind: &str,
    dry_run: bool,
    extra_refs: &[(String, Option<String>, Option<String>)],
) -> Result<LocalMutationResult> {
    let old = current_record(repo, selector)?;
    ensure_active(&old.metadata)?;
    for step in &mut metadata.steps {
        if step.id.is_empty() {
            step.id = Uuid::new_v4().to_string();
        }
    }
    let record = if dry_run {
        None
    } else {
        Some(persistence::write_record(
            repo,
            &metadata,
            &user_metadata,
            &old.lifecycle,
            None,
            Some(&old.record_oid),
            false,
        )?)
    };
    let planned_record_oid = record
        .as_ref()
        .map(|record| record.record_oid.clone())
        .unwrap_or_else(|| old.record_oid.clone());
    let mut plan = MutationPlan::new(kind, Some(metadata.id.clone()))
        .expected_record(Some(old.record_oid.clone()));
    let record_ref = StaircaseRefs::state_record(&metadata.id);
    plan.update(
        record_ref.clone(),
        Some(old.record_oid.clone()),
        Some(planned_record_oid.clone()),
    );
    let public_ref = StaircaseRefs::public(&selector.metadata().name);
    if repo.resolve_ref_opt(&public_ref)?.as_deref() == Some(old.record_oid.as_str()) {
        plan.update(
            public_ref.clone(),
            Some(old.record_oid.clone()),
            Some(planned_record_oid),
        );
    }

    let old_steps: BTreeMap<_, _> = old
        .metadata
        .steps
        .iter()
        .map(|step| (step.id.clone(), step))
        .collect();
    let new_steps: BTreeMap<_, _> = metadata
        .steps
        .iter()
        .map(|step| (step.id.clone(), step))
        .collect();
    for (id, old_step) in &old_steps {
        if !new_steps.contains_key(id) {
            plan.update(
                StaircaseRefs::state_step(&metadata.id, id),
                Some(old_step.cut.clone()),
                None,
            );
        }
    }
    for (id, step) in &new_steps {
        plan.update(
            StaircaseRefs::state_step(&metadata.id, id),
            old_steps.get(id).map(|old_step| old_step.cut.clone()),
            Some(step.cut.clone()),
        );
    }
    let overridden_refs = extra_refs
        .iter()
        .map(|(reference, _, _)| reference.clone())
        .collect();
    add_branch_permutation(repo, &old.metadata, &metadata, &mut plan, &overridden_refs)?;
    for (reference, expected, planned) in extra_refs {
        plan.update(reference.clone(), expected.clone(), planned.clone());
    }
    let changed_refs = plan
        .refs
        .iter()
        .map(|edit| edit.reference.clone())
        .collect();
    plan.publish(repo, dry_run)?;
    Ok(result(
        kind,
        &metadata,
        record.map(|record| record.record_oid),
        dry_run,
        changed_refs,
    ))
}

fn add_branch_permutation(
    repo: &GitRepo,
    old: &StaircaseMetadata,
    new: &StaircaseMetadata,
    plan: &mut MutationPlan,
    overridden_refs: &BTreeSet<String>,
) -> Result<()> {
    let old_owned: BTreeMap<String, String> = old
        .steps
        .iter()
        .filter_map(|step| {
            step.branch
                .as_ref()
                .map(|branch| (local_ref(branch), step.cut.clone()))
        })
        .collect();
    let new_owned: BTreeMap<String, String> = new
        .steps
        .iter()
        .filter_map(|step| {
            step.branch
                .as_ref()
                .map(|branch| (local_ref(branch), step.cut.clone()))
        })
        .collect();
    let all_refs: BTreeSet<_> = old_owned.keys().chain(new_owned.keys()).cloned().collect();
    let current_worktree = repo
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone());
    let checked_out: BTreeMap<_, _> = repo
        .worktrees()?
        .into_iter()
        .filter(|worktree| {
            worktree
                .path
                .canonicalize()
                .unwrap_or_else(|_| worktree.path.clone())
                != current_worktree
        })
        .filter_map(|worktree| worktree.branch.map(|branch| (branch, worktree.path)))
        .collect();
    for reference in all_refs {
        if overridden_refs.contains(&reference) {
            continue;
        }
        let actual = repo.resolve_ref_opt(&reference)?;
        let old_oid = old_owned.get(&reference);
        if let Some(expected) = old_oid {
            let already_at_planned = new_owned
                .get(&reference)
                .is_some_and(|planned| actual.as_deref() == Some(planned.as_str()));
            if actual.as_deref() != Some(expected.as_str()) && !already_at_planned {
                return Err(StaircaseError::RefCollision {
                    reference,
                    expected: expected.clone(),
                    actual: actual.unwrap_or_else(|| "<missing>".into()),
                });
            }
        } else if actual.is_some() {
            return Err(StaircaseError::RefCollision {
                reference,
                expected: "<missing>".into(),
                actual: actual.expect("checked"),
            });
        }
        let planned = new_owned.get(&reference).cloned();
        if actual != planned {
            if let Some(path) = checked_out.get(&reference) {
                return Err(StaircaseError::UnsupportedTopology {
                    operation: "branch-layout".into(),
                    reason: format!(
                        "branch {} is checked out in worktree {}; move or detach that worktree first",
                        reference,
                        path.display()
                    ),
                });
            }
        }
        if actual != planned {
            plan.update(reference, actual, planned);
        }
    }
    Ok(())
}

fn resolve_linear_append(repo: &GitRepo, old_top: &str, range: &str) -> Result<Vec<String>> {
    let commits = repo
        .run(&["rev-list", "--reverse", range])?
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if commits.is_empty() {
        return Err(StaircaseError::InvalidStructure(
            "append range contains no commits".into(),
        ));
    }
    let mut predecessor = repo.resolve_commit(old_top)?;
    for commit in &commits {
        let parents = repo
            .run(&["show", "-s", "--format=%P", commit])?
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if parents.len() != 1 {
            return Err(StaircaseError::UnsupportedTopology {
                operation: "append".into(),
                reason: format!("commit {} is a merge or root commit", commit),
            });
        }
        if parents[0] != predecessor {
            return Err(StaircaseError::UnsupportedTopology {
                operation: "append".into(),
                reason: format!(
                    "range is not a contiguous dependency chain above current top {}",
                    old_top
                ),
            });
        }
        predecessor = commit.clone();
    }
    Ok(commits)
}

fn current_record(repo: &GitRepo, selector: &ResolvedSelector) -> Result<StaircaseRecord> {
    if !selector.is_managed() {
        return Err(StaircaseError::Other(
            "operation requires a managed staircase".into(),
        ));
    }
    current_record_for_metadata(repo, selector.metadata())
}

fn current_record_for_metadata(
    repo: &GitRepo,
    metadata: &StaircaseMetadata,
) -> Result<StaircaseRecord> {
    let reference = if metadata
        .lifecycle
        .as_ref()
        .is_some_and(|lifecycle| lifecycle.state == LifecycleState::Archived)
    {
        StaircaseRefs::archive_record(&metadata.id)
    } else {
        StaircaseRefs::state_record(&metadata.id)
    };
    persistence::read_record(repo, &reference)
}

fn require_managed(selector: &ResolvedSelector) -> Result<&StaircaseMetadata> {
    if !selector.is_managed() {
        return Err(StaircaseError::Other(
            "operation requires a managed staircase".into(),
        ));
    }
    ensure_active(selector.metadata())?;
    Ok(selector.metadata())
}

fn ensure_active(metadata: &StaircaseMetadata) -> Result<()> {
    if metadata
        .lifecycle
        .as_ref()
        .is_some_and(|lifecycle| lifecycle.state == LifecycleState::Archived)
    {
        return Err(StaircaseError::Other(
            "staircase is archived; unarchive it before mutation".into(),
        ));
    }
    Ok(())
}

fn expected_layout_names(metadata: &StaircaseMetadata) -> Result<Option<Vec<String>>> {
    if metadata.primary_branch_layout.is_none() {
        return Ok(None);
    }
    if metadata.primary_branch_layout.as_deref() != Some("sequential-v1") {
        return Err(StaircaseError::Other(
            "unsupported branch layout profile".into(),
        ));
    }
    let base = metadata
        .branch_layout_base
        .as_ref()
        .ok_or_else(|| StaircaseError::Other("sequential-v1 layout is missing its base".into()))?;
    Ok(Some(
        (0..metadata.steps.len())
            .map(|index| sequential_name(index, metadata.steps.len(), base))
            .collect(),
    ))
}

fn apply_layout_names(metadata: &mut StaircaseMetadata) -> Result<()> {
    let names = expected_layout_names(metadata)?.unwrap_or_default();
    for (step, name) in metadata.steps.iter_mut().zip(names) {
        step.branch = Some(name);
    }
    Ok(())
}

fn sequential_name(index: usize, total: usize, base: &str) -> String {
    if index + 1 == total {
        base.into()
    } else {
        format!("{}-{}", base, index + 1)
    }
}

fn extension_map(metadata: &StaircaseUserMetadata, key: &str) -> BTreeMap<String, Value> {
    metadata
        .extensions
        .get(&format!("git-staircase.{}", key))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn validate_policy_key(key: &str) -> Result<()> {
    let allowed = [
        "discovery.",
        "verification.",
        "review.",
        "landing.",
        "merge.",
        "retention.",
        "layout.",
        "github.",
        "gerrit.",
        "repo.",
    ];
    if !allowed.iter().any(|prefix| key.starts_with(prefix)) {
        return Err(StaircaseError::Other(format!(
            "policy key '{}' is not in a registered namespace",
            key
        )));
    }
    Ok(())
}

fn canonical_override_value(repo: &GitRepo, kind: &str, value: &str) -> Result<String> {
    match kind {
        "include-ref" | "exclude-ref" => {
            if !value.starts_with("refs/") {
                return Err(StaircaseError::Other(
                    "discovery ref overrides require a full refname".into(),
                ));
            }
            repo.run(&["check-ref-format", value])?;
            if kind == "include-ref" {
                repo.resolve_commit(value)?;
            }
            Ok(value.into())
        }
        "add-cut" => repo.resolve_commit(value),
        "ignore-cut" if value.starts_with("refs/") => {
            repo.run(&["check-ref-format", value])?;
            Ok(value.into())
        }
        "ignore-cut" => repo.resolve_commit(value),
        _ => Err(StaircaseError::Other(format!(
            "unknown discovery override kind '{}'",
            kind
        ))),
    }
}

fn validate_local_branch(repo: &GitRepo, branch: &str) -> Result<()> {
    repo.run(&["check-ref-format", "--branch", branch])?;
    Ok(())
}

fn validate_staircase_name(repo: &GitRepo, name: &str) -> Result<()> {
    repo.run(&["check-ref-format", &StaircaseRefs::public(name)])?;
    Ok(())
}

fn check_name_available(repo: &GitRepo, name: &str, same_lineage: Option<&str>) -> Result<()> {
    let reference = StaircaseRefs::public(name);
    if let Some(oid) = repo.resolve_ref_opt(&reference)? {
        let lineage = persistence::read_record(repo, &oid)
            .map(|record| record.metadata.id)
            .unwrap_or_default();
        if same_lineage != Some(lineage.as_str()) {
            return Err(StaircaseError::RefCollision {
                reference,
                expected: "<missing>".into(),
                actual: oid,
            });
        }
    }
    for archived in persistence::list_archived_staircases(repo)? {
        if archived.name == name
            && archived
                .lifecycle
                .as_ref()
                .is_some_and(|lifecycle| lifecycle.name_reserved)
            && same_lineage != Some(archived.id.as_str())
        {
            return Err(StaircaseError::RefCollision {
                reference,
                expected: "<unreserved>".into(),
                actual: format!("reserved-by:{}", archived.id),
            });
        }
    }
    Ok(())
}

fn local_ref(branch: &str) -> String {
    if branch.starts_with("refs/heads/") {
        branch.into()
    } else {
        format!("refs/heads/{}", branch)
    }
}

fn result(
    kind: &str,
    metadata: &StaircaseMetadata,
    record_oid: Option<String>,
    dry_run: bool,
    changed_refs: Vec<String>,
) -> LocalMutationResult {
    LocalMutationResult {
        schema: "git-staircase/local-mutation-result".into(),
        version: 1,
        kind: kind.into(),
        staircase_id: metadata.id.clone(),
        staircase_name: metadata.name.clone(),
        record_oid,
        dry_run,
        changed_refs,
    }
}

use crate::presentation::{Presentation, ToPresentation, UsePresentation};

impl ToPresentation for LocalMutationResult {
    fn to_presentation(&self) -> Presentation {
        let mut children = vec![
            Presentation::Field {
                label: "kind".to_string(),
                value: self.kind.clone(),
            },
            Presentation::Field {
                label: "staircase".to_string(),
                value: self.staircase_name.clone(),
            },
        ];
        if let Some(ref oid) = self.record_oid {
             children.push(Presentation::Field {
                label: "record oid".to_string(),
                value: oid[..7].to_string(),
            });
        }
        if self.dry_run {
            children.push(Presentation::Plain("(dry run)".to_string()));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Operation '{}' completed successfully:", self.kind),
                children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.kind.clone(),
                self.staircase_name.clone(),
                self.record_oid.clone().unwrap_or_default(),
            ]))),
        ])
    }
}

impl ToPresentation for LayoutState {
    fn to_presentation(&self) -> Presentation {
        let mut branches = vec![];
        for b in &self.branches {
            branches.push(vec![
                b.step_name.clone(),
                b.expected_oid[..7].to_string(),
                b.actual_oid.as_deref().unwrap_or("none")[..7.min(b.actual_oid.as_deref().unwrap_or("none").len())].to_string(),
            ]);
        }
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Layout state for staircase {}:", self.staircase_id),
                children: vec![
                    Presentation::Field { label: "profile".to_string(), value: self.profile.clone().unwrap_or("none".into()) },
                    Presentation::Field { label: "base".to_string(), value: self.base.clone().unwrap_or("none".into()) },
                    Presentation::Field { label: "state".to_string(), value: self.state.clone() },
                    Presentation::Table { name: Some("branches".into()), rows: branches },
                ],
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "layout".to_string(),
                self.staircase_id.clone(),
                self.state.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for DiscoveryOverride {
    fn to_presentation(&self) -> Presentation {
        Presentation::Record(vec![self.id.clone(), self.kind.clone(), self.value.clone()])
    }
}
impl UsePresentation for LocalMutationResult {}
impl UsePresentation for LayoutState {}
impl UsePresentation for DiscoveryOverride {}

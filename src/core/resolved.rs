use super::persistence;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{StaircaseFamily, StaircaseMetadata, Step};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "management", rename_all = "kebab-case")]
pub enum ResolvedStaircase {
    Managed(StaircaseMetadata),
    Implicit(StaircaseMetadata),
    ImplicitFamily(StaircaseFamily),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSelector {
    pub staircase: ResolvedStaircase,
    pub step_index: Option<usize>,
}

impl Deref for ResolvedSelector {
    type Target = ResolvedStaircase;

    fn deref(&self) -> &Self::Target {
        &self.staircase
    }
}

impl ResolvedSelector {
    pub fn metadata(&self) -> &StaircaseMetadata {
        self.staircase.metadata()
    }
}

impl ResolvedStaircase {
    pub fn metadata(&self) -> &StaircaseMetadata {
        match self {
            ResolvedStaircase::Managed(s) => s,
            ResolvedStaircase::Implicit(s) => s,
            ResolvedStaircase::ImplicitFamily(_) => panic!("Family does not have linear metadata"),
        }
    }

    pub fn is_managed(&self) -> bool {
        matches!(self, ResolvedStaircase::Managed(_))
    }

    pub fn add_step(
        &self,
        repo: &GitRepo,
        index: usize,
        step: Step,
        no_ref: bool,
    ) -> Result<ResolvedStaircase> {
        let managed = ensure_managed(repo, self)?;
        let mut metadata = managed.metadata().clone();
        if no_ref {
            metadata.primary_branch_layout = None;
            metadata.branch_layout_base = None;
        }
        metadata.steps.insert(index, step.clone());

        validate_renumbering(repo, managed.metadata(), &mut metadata)?;
        publish_managed(repo, &managed, metadata)
    }

    /// Remove a step at the given index and persist the change.
    pub fn remove_step(&self, repo: &GitRepo, index: usize) -> Result<ResolvedStaircase> {
        let managed = ensure_managed(repo, self)?;
        let mut metadata = managed.metadata().clone();
        metadata.steps.remove(index);
        validate_renumbering(repo, managed.metadata(), &mut metadata)?;
        publish_managed(repo, &managed, metadata)
    }

    /// Update a step's OID and persist.
    pub fn update_step_oid(
        &self,
        repo: &GitRepo,
        index: usize,
        new_oid: String,
    ) -> Result<ResolvedStaircase> {
        let managed = ensure_managed(repo, self)?;
        let mut metadata = managed.metadata().clone();
        metadata.steps[index].cut = new_oid.clone();
        publish_managed(repo, &managed, metadata)
    }

    /// Update the entire metadata and persist it.
    /// This is used for operations like reorder where multiple things change.
    pub fn commit_metadata(
        &self,
        repo: &GitRepo,
        metadata: StaircaseMetadata,
    ) -> Result<ResolvedStaircase> {
        let managed = ensure_managed(repo, self)?;
        let mut metadata = metadata;
        metadata.id = managed.metadata().id.clone();
        validate_renumbering(repo, managed.metadata(), &mut metadata)?;
        publish_managed(repo, &managed, metadata)
    }
}

fn ensure_managed(repo: &GitRepo, staircase: &ResolvedStaircase) -> Result<ResolvedStaircase> {
    if staircase.is_managed() {
        Ok(staircase.clone())
    } else {
        Ok(ResolvedStaircase::Managed(adopt(
            repo,
            staircase.metadata(),
        )?))
    }
}

fn publish_managed(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    metadata: StaircaseMetadata,
) -> Result<ResolvedStaircase> {
    let selector = ResolvedSelector {
        staircase: staircase.clone(),
        step_index: None,
    };
    super::local::publish_metadata(repo, &selector, metadata.clone(), "structure", false)?;
    Ok(ResolvedStaircase::Managed(metadata))
}

pub fn is_clean(repo: &GitRepo, staircase: &StaircaseMetadata) -> Result<bool> {
    let target_oid = match repo.resolve_commit(&staircase.target) {
        Ok(oid) => oid,
        Err(_) => return Ok(false),
    };
    let mut oids = vec![target_oid.as_str()];
    for step in &staircase.steps {
        oids.push(step.cut.as_str());
    }
    let _ = repo.preload_ancestry(&oids);

    let mut last_cut = target_oid;
    for step in &staircase.steps {
        let current_cut = match repo.resolve_commit(&step.cut) {
            Ok(oid) => oid,
            Err(_) => return Ok(false),
        };
        if current_cut == last_cut {
            return Ok(false);
        }
        if !repo.is_ancestor(&last_cut, &current_cut)? {
            return Ok(false);
        }
        last_cut = current_cut;
    }
    Ok(true)
}

pub fn adopt(repo: &GitRepo, staircase: &StaircaseMetadata) -> Result<StaircaseMetadata> {
    let target = match repo.resolve_symbolic_full_name(&staircase.target) {
        Ok(t) => t,
        Err(_) => repo.resolve_commit(&staircase.target)?,
    };
    let target_oid = repo.resolve_commit(&staircase.target)?;
    let mut staircase = staircase.clone();
    staircase.target = target;
    if staircase.id.starts_with("implicit@") {
        staircase.id = Uuid::new_v4().to_string();
    }
    for step in &mut staircase.steps {
        if step.id.is_empty() {
            step.id = Uuid::new_v4().to_string();
        }
    }
    let mut oids = vec![target_oid.as_str()];
    for step in &staircase.steps {
        oids.push(step.cut.as_str());
    }
    let _ = repo.preload_ancestry(&oids);
    let mut last_cut = target_oid;
    for step in &staircase.steps {
        let current_cut = repo.resolve_commit(&step.cut)?;
        if current_cut == last_cut {
            return Err(StaircaseError::InvalidStructure(format!(
                "Step \"{}\" cut \"{}\" is identical to its predecessor; every step must be non-empty",
                step.name, step.cut
            )));
        }
        if !repo.is_ancestor(&last_cut, &current_cut)? {
            return Err(StaircaseError::InvalidStructure(format!(
                "Step \"{}\" cut \"{}\" is not a descendant of its predecessor",
                step.name, step.cut
            )));
        }
        last_cut = current_cut;
    }

    persistence::write_metadata(repo, &staircase)?;
    Ok(staircase)
}

fn plan_renumbering(
    repo: &GitRepo,
    old_metadata: &StaircaseMetadata,
    new_metadata: &mut StaircaseMetadata,
) -> Result<Vec<String>> {
    if new_metadata.primary_branch_layout.as_deref() != Some("sequential-v1") {
        return Ok(vec![]);
    }
    let Some(ref base) = new_metadata.branch_layout_base else {
        return Ok(vec![]);
    };

    let n = new_metadata.steps.len();
    let mut expected_branches = Vec::new();
    for i in 0..n {
        let name = sequential_branch_name(i, n, base);
        expected_branches.push(name.clone());
    }
    let new_cuts_by_step = new_metadata
        .steps
        .iter()
        .map(|step| {
            (
                if step.id.is_empty() {
                    step.name.as_str()
                } else {
                    step.id.as_str()
                },
                step.cut.as_str(),
            )
        })
        .collect::<HashMap<_, _>>();
    let old_owned = old_metadata
        .steps
        .iter()
        .filter_map(|step| {
            step.branch.as_ref().map(|branch| {
                (
                    format!("refs/heads/{}", branch),
                    new_cuts_by_step
                        .get(if step.id.is_empty() {
                            step.name.as_str()
                        } else {
                            step.id.as_str()
                        })
                        .map(|cut| (*cut).to_string())
                        .unwrap_or_else(|| step.cut.clone()),
                )
            })
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for (step, branch) in new_metadata.steps.iter_mut().zip(expected_branches) {
        step.branch = Some(branch);
    }
    let new_owned = new_metadata
        .steps
        .iter()
        .filter_map(|step| {
            step.branch
                .as_ref()
                .map(|branch| (format!("refs/heads/{}", branch), step.cut.clone()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let refs = old_owned
        .keys()
        .chain(new_owned.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let checked_out = repo
        .worktrees()?
        .into_iter()
        .filter_map(|worktree| worktree.branch.map(|branch| (branch, worktree.path)))
        .collect::<HashMap<_, _>>();
    let mut commands = Vec::new();
    for reference in refs {
        let actual = repo.resolve_ref_opt(&reference)?;
        let old = old_owned.get(&reference);
        let planned = new_owned.get(&reference);
        if let Some(expected) = old {
            let already_planned =
                planned.is_some_and(|oid| actual.as_deref() == Some(oid.as_str()));
            if actual.as_deref() != Some(expected.as_str()) && !already_planned {
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
        if actual.as_ref() == planned {
            continue;
        }
        if let Some(path) = checked_out.get(&reference) {
            return Err(StaircaseError::UnsupportedTopology {
                operation: "branch-layout".into(),
                reason: format!(
                    "branch {} is checked out in worktree {}",
                    reference,
                    path.display()
                ),
            });
        }
        match (actual, planned) {
            (None, Some(new)) => commands.push(format!("create {} {}", reference, new)),
            (Some(old), Some(new)) => {
                commands.push(format!("update {} {} {}", reference, new, old))
            }
            (Some(old), None) => commands.push(format!("delete {} {}", reference, old)),
            (None, None) => {}
        }
    }
    Ok(commands)
}

pub(crate) fn validate_renumbering(
    repo: &GitRepo,
    old_metadata: &StaircaseMetadata,
    new_metadata: &mut StaircaseMetadata,
) -> Result<()> {
    plan_renumbering(repo, old_metadata, new_metadata).map(|_| ())
}

fn sequential_branch_name(index: usize, total: usize, base: &str) -> String {
    if index == total - 1 {
        base.to_string()
    } else {
        format!("{}-{}", base, index + 1)
    }
}

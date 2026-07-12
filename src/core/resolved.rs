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
        let mut metadata = self.metadata().clone();
        if no_ref {
            metadata.primary_branch_layout = None;
            metadata.branch_layout_base = None;
        }
        metadata.steps.insert(index, step.clone());

        let mut metadata = metadata;
        let commands = plan_renumbering(repo, self.metadata(), &mut metadata)?;
        repo.update_refs_transaction(&commands)?;

        if self.is_managed() {
            persistence::write_metadata(repo, &metadata)?;
            repo.update_step_ref(&metadata.id, &step.name, &step.cut)?;
            Ok(ResolvedStaircase::Managed(metadata))
        } else {
            let has_branch = metadata.steps[index].branch.is_some();
            if has_branch {
                if commands.is_empty() {
                    if let Some(ref branch) = step.branch {
                        repo.update_branch(branch, &step.cut)?;
                    }
                }
                Ok(ResolvedStaircase::Implicit(metadata))
            } else {
                let metadata = adopt(repo, &metadata)?;
                Ok(ResolvedStaircase::Managed(metadata))
            }
        }
    }

    /// Remove a step at the given index and persist the change.
    pub fn remove_step(&self, repo: &GitRepo, index: usize) -> Result<ResolvedStaircase> {
        let mut metadata = self.metadata().clone();
        let removed = metadata.steps.remove(index);

        let mut metadata = metadata;
        let commands = plan_renumbering(repo, self.metadata(), &mut metadata)?;
        repo.update_refs_transaction(&commands)?;

        if self.is_managed() {
            persistence::write_metadata(repo, &metadata)?;
            repo.delete_step_ref(&metadata.id, &removed.name)?;
            Ok(ResolvedStaircase::Managed(metadata))
        } else {
            // If all remaining steps have branches, it can stay implicit.
            // But if it becomes stale, it must be managed.
            if metadata.steps.iter().all(|s| s.branch.is_some()) && is_clean(repo, &metadata)? {
                Ok(ResolvedStaircase::Implicit(metadata))
            } else {
                let metadata = adopt(repo, &metadata)?;
                Ok(ResolvedStaircase::Managed(metadata))
            }
        }
    }

    /// Update a step's OID and persist.
    pub fn update_step_oid(
        &self,
        repo: &GitRepo,
        index: usize,
        new_oid: String,
    ) -> Result<ResolvedStaircase> {
        let mut metadata = self.metadata().clone();
        metadata.steps[index].cut = new_oid.clone();

        if self.is_managed() {
            repo.update_step_ref(&metadata.id, &metadata.steps[index].name, &new_oid)?;
            persistence::write_metadata(repo, &metadata)?;
            Ok(ResolvedStaircase::Managed(metadata))
        } else {
            if let Some(ref branch) = metadata.steps[index].branch {
                repo.update_branch(branch, &new_oid)?;
                // Check if the entire staircase is still clean.
                // If we are restacking, it might be transiently stale.
                // But update_step_oid is usually called by restack which then calls commit_metadata.
                // To avoid premature adoption during restack, we allow it to stay implicit if it has a branch.
                Ok(ResolvedStaircase::Implicit(metadata))
            } else {
                let metadata = adopt(repo, &metadata)?;
                Ok(ResolvedStaircase::Managed(metadata))
            }
        }
    }

    /// Update the entire metadata and persist it.
    /// This is used for operations like reorder where multiple things change.
    pub fn commit_metadata(
        &self,
        repo: &GitRepo,
        metadata: StaircaseMetadata,
    ) -> Result<ResolvedStaircase> {
        let mut metadata = metadata;
        let commands = plan_renumbering(repo, self.metadata(), &mut metadata)?;
        repo.update_refs_transaction(&commands)?;

        if self.is_managed() {
            persistence::write_metadata(repo, &metadata)?;
            for step in &metadata.steps {
                repo.update_step_ref(&metadata.id, &step.name, &step.cut)?;
            }
            Ok(ResolvedStaircase::Managed(metadata))
        } else {
            // An implicit staircase can stay implicit if it remains discoverable (all steps have branches)
            // and it is clean. Stale staircases must be managed.
            if metadata.steps.iter().all(|s| s.branch.is_some()) && is_clean(repo, &metadata)? {
                Ok(ResolvedStaircase::Implicit(metadata))
            } else {
                let metadata = adopt(repo, &metadata)?;
                Ok(ResolvedStaircase::Managed(metadata))
            }
        }
    }
}

pub fn is_clean(repo: &GitRepo, staircase: &StaircaseMetadata) -> Result<bool> {
    let target_oid = match repo.resolve_commit(&staircase.target) {
        Ok(oid) => oid,
        Err(_) => return Ok(false),
    };
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
    for step in &staircase.steps {
        repo.update_step_ref(&staircase.id, &step.name, &step.cut)?;
    }
    Ok(staircase)
}

fn plan_renumbering(
    _repo: &GitRepo,
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
    let mut new_refs = std::collections::HashSet::new();
    let mut expected_branches = Vec::new();
    for i in 0..n {
        let name = sequential_branch_name(i, n, base);
        expected_branches.push(name.clone());
        new_refs.insert(format!("refs/heads/{}", name));
    }

    let mut commands = Vec::new();
    
    // Helper to get match key for a step
    let step_key = |s: &Step| {
        if !s.id.is_empty() {
            s.id.clone()
        } else {
            s.name.clone()
        }
    };

    let mut old_steps_by_key = HashMap::new();
    for s in &old_metadata.steps {
        old_steps_by_key.insert(step_key(s), s);
    }

    // Map to find the current OID of an old branch in Git.
    let mut old_branch_current_oid = HashMap::new();
    for step in &new_metadata.steps {
        if let Some(old_step) = old_steps_by_key.get(&step_key(step)) {
            if let Some(ref old_branch) = old_step.branch {
                old_branch_current_oid.insert(old_branch.clone(), step.cut.clone());
            }
        }
    }
    for old_step in &old_metadata.steps {
        let key = step_key(old_step);
        if !new_metadata.steps.iter().any(|s| step_key(s) == key) {
             if let Some(ref old_branch) = old_step.branch {
                 old_branch_current_oid.insert(old_branch.clone(), old_step.cut.clone());
             }
        }
    }

    for i in 0..n {
        let expected_name = &expected_branches[i];
        let new_ref = format!("refs/heads/{}", expected_name);
        let step = &mut new_metadata.steps[i];
        let new_oid = &step.cut;

        let mut old_ref = None;
        if let Some(old_step) = old_steps_by_key.get(&step_key(step)) {
            if let Some(ref old_branch) = old_step.branch {
                old_ref = Some(format!("refs/heads/{}", old_branch));
            }
        }

        step.branch = Some(expected_name.clone());

        match old_ref {
            Some(ref_name) => {
                if ref_name == new_ref {
                    commands.push(format!("update {} {} {}", new_ref, new_oid, new_oid));
                } else {
                    if !new_refs.contains(&ref_name) {
                        commands.push(format!("delete {} {}", ref_name, new_oid));
                    }
                    
                    let current_oid_of_new = old_branch_current_oid.get(expected_name);
                    if let Some(curr_oid) = current_oid_of_new {
                        commands.push(format!("update {} {} {}", new_ref, new_oid, curr_oid));
                    } else {
                        commands.push(format!("create {} {}", new_ref, new_oid));
                    }
                }
            }
            None => {
                let current_oid_of_new = old_branch_current_oid.get(expected_name);
                if let Some(curr_oid) = current_oid_of_new {
                    commands.push(format!("update {} {} {}", new_ref, new_oid, curr_oid));
                } else {
                    commands.push(format!("create {} {}", new_ref, new_oid));
                }
            }
        }
    }

    // Handle dropped steps
    for old_step in &old_metadata.steps {
        let key = step_key(old_step);
        if !new_metadata.steps.iter().any(|s| step_key(s) == key) {
            if let Some(ref old_branch) = old_step.branch {
                let ref_name = format!("refs/heads/{}", old_branch);
                if !new_refs.contains(&ref_name) {
                    commands.push(format!("delete {} {}", ref_name, old_step.cut));
                }
            }
        }
    }

    Ok(commands)
}

fn sequential_branch_name(index: usize, total: usize, base: &str) -> String {
    if index == total - 1 {
        base.to_string()
    } else {
        format!("{}-{}", base, index + 1)
    }
}


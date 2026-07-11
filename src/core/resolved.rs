use super::persistence;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{StaircaseMetadata, Step, ToHuman, ToPorcelain};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "management", rename_all = "kebab-case")]
pub enum ResolvedStaircase {
    Managed(StaircaseMetadata),
    Implicit(StaircaseMetadata),
}

impl ResolvedStaircase {
    pub fn metadata(&self) -> &StaircaseMetadata {
        match self {
            ResolvedStaircase::Managed(s) => s,
            ResolvedStaircase::Implicit(s) => s,
        }
    }

    pub fn is_managed(&self) -> bool {
        matches!(self, ResolvedStaircase::Managed(_))
    }

    /// Add a step at the given index and persist the change.
    pub fn add_step(&self, repo: &GitRepo, index: usize, step: Step) -> Result<ResolvedStaircase> {
        let mut metadata = self.metadata().clone();
        metadata.steps.insert(index, step.clone());

        if self.is_managed() {
            persistence::write_metadata(repo, &metadata)?;
            repo.update_step_ref(&metadata.id, &step.name, &step.cut)?;
            Ok(ResolvedStaircase::Managed(metadata))
        } else {
            if let Some(ref branch) = step.branch {
                repo.update_branch(branch, &step.cut)?;
                Ok(ResolvedStaircase::Implicit(metadata))
            } else {
                adopt(repo, &metadata)?;
                Ok(ResolvedStaircase::Managed(metadata))
            }
        }
    }

    /// Remove a step at the given index and persist the change.
    pub fn remove_step(&self, repo: &GitRepo, index: usize) -> Result<ResolvedStaircase> {
        let mut metadata = self.metadata().clone();
        let removed = metadata.steps.remove(index);

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
                adopt(repo, &metadata)?;
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
                adopt(repo, &metadata)?;
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
                adopt(repo, &metadata)?;
                Ok(ResolvedStaircase::Managed(metadata))
            }
        }
    }
}

pub fn is_clean(repo: &GitRepo, staircase: &StaircaseMetadata) -> Result<bool> {
    let target_oid = match repo.resolve_ref(&staircase.target) {
        Ok(oid) => oid,
        Err(_) => return Ok(false),
    };
    let mut last_cut = target_oid;
    for step in &staircase.steps {
        let current_cut = match repo.resolve_ref(&step.cut) {
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

pub fn adopt(repo: &GitRepo, staircase: &StaircaseMetadata) -> Result<()> {
    let target_oid = repo.resolve_ref(&staircase.target)?;
    let mut last_cut = target_oid;
    for step in &staircase.steps {
        let current_cut = repo.resolve_ref(&step.cut)?;
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

    persistence::write_metadata(repo, staircase)?;
    for step in &staircase.steps {
        repo.update_step_ref(&staircase.id, &step.name, &step.cut)?;
    }
    Ok(())
}

impl ToPorcelain for ResolvedStaircase {
    fn to_porcelain(&self) -> String {
        let m = self.metadata();
        format!(
            "{}\t{}\t{}\t{}",
            m.name,
            m.id,
            if self.is_managed() {
                "managed"
            } else {
                "implicit"
            },
            m.steps.len()
        )
    }
}

impl ToHuman for ResolvedStaircase {
    fn to_human(&self) -> String {
        let mut out = if self.is_managed() {
            format!("Managed Staircase: {}\n", self.metadata().name)
        } else {
            format!("Implicit Staircase: {}\n", self.metadata().name)
        };
        out.push_str(&self.metadata().to_human());
        out
    }
}

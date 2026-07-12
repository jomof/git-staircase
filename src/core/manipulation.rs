use super::persistence;
use crate::core::ResolvedStaircase;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{LandingPolicy, Step};
use std::collections::{HashMap, HashSet};
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

pub fn split(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    at_commit: &str,
    new_step_name: Option<&str>,
    options: SplitOptions,
) -> Result<()> {
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

pub fn join(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index_1: usize,
    step_index_2: usize,
    options: JoinOptions,
) -> Result<()> {
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

    // Perform ref action
    if let Some(ref branch) = removed_step.branch {
        match options.ref_action {
            JoinRefAction::Delete => {
                let _ = repo.run(&["branch", "-D", branch]);
            }
            JoinRefAction::Rename(ref new_name) => {
                let _ = repo.run(&["branch", "-m", branch, new_name]);
            }
            JoinRefAction::Keep => {
                // Do nothing to the ref.
            }
        }
    }

    let result_rs = staircase.remove_step(repo, low)?;

    if options.ref_action == JoinRefAction::Keep && !result_rs.is_managed() {
        // Force adoption if we kept a boundary ref that is no longer in metadata.
        let metadata = crate::core::adopt(repo, result_rs.metadata())?;
        let _metadata = ResolvedStaircase::Managed(metadata);
    }

    Ok(())
}

struct StaircaseRebaser<'a> {
    repo: &'a GitRepo,
    original_head: Option<String>,
    original_head_oid: String,
    original_branch_oids: HashMap<String, String>,
}

impl<'a> StaircaseRebaser<'a> {
    fn new(repo: &'a GitRepo, steps: &[Step]) -> Result<Self> {
        let original_head = repo.current_branch()?;
        let original_head_oid = repo.resolve_commit("HEAD")?;
        let mut original_branch_oids = HashMap::new();
        for step in steps {
            if let Some(ref branch) = step.branch {
                if let Some(oid) = repo.resolve_commit_opt(&format!("refs/heads/{}", branch))? {
                    original_branch_oids.insert(branch.clone(), oid);
                }
            }
        }
        Ok(Self {
            repo,
            original_head,
            original_head_oid,
            original_branch_oids,
        })
    }

    fn rollback(&self) {
        let _ = self.repo.run(&["rebase", "--abort"]);
        for (branch, oid) in &self.original_branch_oids {
            let _ = self.repo.update_branch(branch, oid);
        }
        self.restore_head_silent();
    }

    fn restore_head_silent(&self) {
        if let Some(ref refname) = self.original_head {
            let target = refname.strip_prefix("refs/heads/").unwrap_or(refname);
            let _ = self.repo.run(&["checkout", target]);
        } else {
            let _ = self.repo.run(&["checkout", &self.original_head_oid]);
        }
    }

    fn finalize(&self) -> Result<()> {
        if let Some(ref refname) = self.original_head {
            let target = refname.strip_prefix("refs/heads/").unwrap_or(refname);
            self.repo.run(&["checkout", target])?;
        } else {
            self.repo.run(&["checkout", &self.original_head_oid])?;
        }
        Ok(())
    }

    fn rebase_step(
        &self,
        step: &Step,
        actual_oid: &str,
        old_parent: &str,
        new_parent: &str,
    ) -> Result<String> {
        let mut rebase_target = actual_oid.to_string();
        if let Some(ref branch_name) = step.branch {
            if self
                .repo
                .resolve_commit_opt(&format!("refs/heads/{}", branch_name))?
                .is_some()
            {
                rebase_target = branch_name.clone();
            }
        }

        self.repo
            .run_interactive(&["rebase", "--onto", new_parent, old_parent, &rebase_target])?;

        self.repo.resolve_commit("HEAD")
    }
}

fn perform_restack(
    repo: &GitRepo,
    rebaser: &StaircaseRebaser,
    steps: &mut [Step],
    base_oid: &str,
    old_parent_oids: &[String],
    options: &RebaseOptions,
) -> Result<Option<usize>> {
    let mut current_base = base_oid.to_string();
    for i in 0..steps.len() {
        let step = &steps[i];
        let actual_oid = repo.resolve_commit(&step.cut)?;
        let old_parent_oid = &old_parent_oids[i];

        let should_rebase =
            &current_base != old_parent_oid || !repo.is_ancestor(&current_base, &actual_oid)?;

        if should_rebase {
            match rebaser.rebase_step(step, &actual_oid, old_parent_oid, &current_base) {
                Ok(new_oid) => {
                    steps[i].cut = new_oid.clone();
                    current_base = new_oid;
                    if options.leave_upper_steps_stale {
                        return Ok(Some(i));
                    }
                }
                Err(e) => {
                    return Err(StaircaseError::Other(format!(
                        "Rebase failed for step '{}'. Please resolve conflicts and run restack again.\nError: {}",
                        step.name, e
                    )));
                }
            }
        } else {
            current_base = actual_oid.clone();
            if steps[i].cut != actual_oid {
                steps[i].cut = actual_oid;
            }
        }
    }
    Ok(None)
}

pub fn reorder(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    new_order: &[usize],
    options: ReorderOptions,
) -> Result<()> {
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

    let mut new_steps = Vec::new();
    for &idx in new_order {
        new_steps.push(old_steps[idx].clone());
    }

    if !options.no_restack {
        let rebaser = StaircaseRebaser::new(repo, &old_steps)?;
        let target_oid = repo.resolve_commit(&metadata.target)?;

        let mut old_parent_oids = Vec::new();
        for i in 0..new_steps.len() {
            let old_idx = new_order[i];
            let actual_oid = repo.resolve_commit(&new_steps[i].cut)?;
            let old_parent_oid = if old_idx == 0 {
                repo.merge_base(&actual_oid, &target_oid)?
            } else {
                old_steps[old_idx - 1].cut.clone()
            };
            old_parent_oids.push(old_parent_oid);
        }

        match perform_restack(
            repo,
            &rebaser,
            &mut new_steps,
            &target_oid,
            &old_parent_oids,
            &RebaseOptions {
                leave_upper_steps_stale: false,
            },
        ) {
            Ok(_) => {
                rebaser.finalize()?;
            }
            Err(e) => {
                rebaser.rollback();
                return Err(e);
            }
        }
    }

    metadata.steps = new_steps;
    staircase.commit_metadata(repo, metadata)?;

    Ok(())
}

pub fn drop(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    options: DropOptions,
) -> Result<()> {
    let metadata = staircase.metadata().clone();
    if step_index >= metadata.steps.len() {
        return Err(StaircaseError::Other(
            "Step index out of bounds".to_string(),
        ));
    }

    let branch_to_delete = if !staircase.is_managed() {
        metadata.steps[step_index].branch.clone()
    } else {
        None
    };

    let mut new_order: Vec<usize> = (0..metadata.steps.len()).collect();
    new_order.remove(step_index);

    reorder(
        repo,
        staircase,
        &new_order,
        ReorderOptions {
            no_restack: !options.restack || options.leave_descendants_stale,
        },
    )?;

    if let Some(branch) = branch_to_delete {
        let _ = repo.run(&["branch", "-D", &branch]);
    }

    Ok(())
}

pub fn move_commits(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    from_step_index: usize,
    to_step_index: usize,
    commits: &[String],
) -> Result<()> {
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

    if to_step_index + 1 == from_step_index {
        let commit_to_move = &commits[0];
        let oid_to_move = repo.resolve_commit(commit_to_move)?;

        let prev_cut = if from_step_index == 0 {
            repo.resolve_commit(&metadata.target)?
        } else {
            metadata.steps[from_step_index - 1].cut.clone()
        };

        let commits_in_from_step =
            repo.commits_between(&prev_cut, &metadata.steps[from_step_index].cut)?;
        if commits_in_from_step.first() == Some(&oid_to_move) {
            metadata.steps[to_step_index].cut = oid_to_move.clone();

            if let Some(ref branch) = metadata.steps[to_step_index].branch {
                repo.update_branch(branch, &oid_to_move)?;
            }

            staircase.commit_metadata(repo, metadata)?;
            return Ok(());
        }
    }

    Err(StaircaseError::Other(
        "Complex move not yet implemented".to_string(),
    ))
}

pub fn restack(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    options: RebaseOptions,
) -> Result<()> {
    let status = crate::core::status::get_status_metadata(
        repo,
        staircase.metadata().clone(),
        !staircase.is_managed(),
    )?;

    if status.is_clean {
        return Ok(());
    }

    let mut metadata = status.metadata.clone();
    let rebaser = StaircaseRebaser::new(repo, &metadata.steps)?;
    let target_oid = repo.resolve_commit(&metadata.target)?;

    let mut old_parent_oids = Vec::new();
    let original_cuts: Vec<String> = status
        .metadata
        .steps
        .iter()
        .map(|s| s.cut.clone())
        .collect();

    for i in 0..metadata.steps.len() {
        let step_status = &status.steps[i];
        let actual_oid = match &step_status.actual_oid {
            Some(oid) => oid.clone(),
            None => metadata.steps[i].cut.clone(),
        };
        // Update metadata.steps[i].cut to actual_oid so perform_restack uses it.
        metadata.steps[i].cut = actual_oid.clone();

        let old_parent_oid = if i == 0 {
            repo.merge_base(&actual_oid, &target_oid)?
        } else {
            original_cuts[i - 1].clone()
        };
        old_parent_oids.push(old_parent_oid);
    }

    match perform_restack(
        repo,
        &rebaser,
        &mut metadata.steps,
        &target_oid,
        &old_parent_oids,
        &options,
    ) {
        Ok(stop_index) => {
            rebaser.finalize()?;

            if let Some(idx) = stop_index {
                for i in (idx + 1)..metadata.steps.len() {
                    metadata.steps[i].cut = original_cuts[i].clone();
                }
            }

            staircase.commit_metadata(repo, metadata)?;
            Ok(())
        }
        Err(e) => {
            rebaser.rollback();
            Err(e)
        }
    }
}

pub fn rebase(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    onto: &str,
    options: RebaseOptions,
) -> Result<()> {
    let mut metadata = staircase.metadata().clone();
    metadata.target = onto.to_string();

    let updated_rs = if staircase.is_managed() {
        staircase.commit_metadata(repo, metadata)?
    } else {
        ResolvedStaircase::Implicit(metadata)
    };
    restack(repo, &updated_rs, options)
}

pub fn delete(repo: &GitRepo, id: &str, delete_branches: bool) -> Result<()> {
    let metadata = persistence::read_metadata(repo, id)?;

    if delete_branches {
        for step in &metadata.steps {
            if let Some(ref branch) = step.branch {
                let _ = repo.run(&["branch", "-D", branch]);
            }
        }
    }

    persistence::delete_staircase_refs(repo, id, &metadata.name)?;
    Ok(())
}

pub struct LandOptions {
    pub policy: Option<LandingPolicy>,
}

pub fn land(repo: &GitRepo, staircase: &ResolvedStaircase, options: LandOptions) -> Result<()> {
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
        match policy {
            LandingPolicy::Stepwise => {
                for step in &metadata.steps {
                    repo.run(&["update-ref", &metadata.target, &step.cut])?;
                }
            }
            LandingPolicy::AggregateOnly | LandingPolicy::Either => {
                repo.run(&["update-ref", &metadata.target, top_cut])?;
            }
        }
    } else {
        return Err(StaircaseError::Other(format!(
            "Target {} is not a ref, cannot land",
            metadata.target
        )));
    }

    Ok(())
}

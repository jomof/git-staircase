use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{StaircaseMetadata, Step};
use crate::core::ResolvedStaircase;

pub fn split(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    at_commit: &str,
    new_step_name: Option<&str>,
) -> Result<()> {
    if step_index >= staircase.metadata().steps.len() {
        return Err(StaircaseError::InvalidStructure(format!(
            "Step index {} out of bounds",
            step_index
        )));
    }

    let at_oid = repo.resolve_ref(at_commit)?;
    let cut_oid = &staircase.metadata().steps[step_index].cut;
    let prev_cut_oid = if step_index == 0 {
        repo.resolve_ref(&staircase.metadata().target)?
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
        name: name.clone(),
        cut: at_oid.clone(),
        branch: if staircase.is_managed() {
            None
        } else {
            new_step_name.map(|n| n.to_string())
        },
    };

    staircase.add_step(repo, step_index, new_step)?;
    Ok(())
}

pub fn join(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index_1: usize,
    step_index_2: usize,
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

    staircase.remove_step(repo, low)?;
    Ok(())
}

pub fn reorder(repo: &GitRepo, staircase: &ResolvedStaircase, new_order: &[usize]) -> Result<()> {
    let mut metadata = staircase.metadata().clone();
    let old_steps = metadata.steps.clone();

    let mut seen = std::collections::HashSet::new();
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

    let target_oid = repo.resolve_ref(&metadata.target)?;
    let mut current_base = target_oid;

    let mut success_count = 0;
    let mut rebase_err = None;

    for i in 0..new_steps.len() {
        let step = &new_steps[i];
        let actual_oid = repo.resolve_ref(&step.cut)?;

        let old_idx = new_order[i];
        let old_parent_oid = if old_idx == 0 {
            repo.merge_base(&actual_oid, &repo.resolve_ref(&metadata.target)?)?
        } else {
            old_steps[old_idx - 1].cut.clone()
        };

        if let Some(ref branch_name) = step.branch {
            if current_base != old_parent_oid {
                match repo.run_interactive(&[
                    "rebase",
                    "--onto",
                    &current_base,
                    &old_parent_oid,
                    branch_name,
                ]) {
                    Ok(_) => {
                        let new_oid = repo.resolve_ref(&format!("refs/heads/{}", branch_name))?;
                        new_steps[i].cut = new_oid.clone();
                        current_base = new_oid;
                        success_count += 1;
                    }
                    Err(e) => {
                        rebase_err = Some(e);
                        break;
                    }
                }
            } else {
                current_base = actual_oid;
                success_count += 1;
            }
        } else {
            return Err(StaircaseError::Other(format!(
                "Step '{}' has no branch; reshaping requires branches",
                step.name
            )));
        }
    }

    if success_count > 0 {
        metadata.steps = new_steps;
        staircase.commit_metadata(repo, metadata)?;
    }

    if let Some(err) = rebase_err {
        return Err(StaircaseError::Other(format!(
            "Reorder failed: {}", err
        )));
    }

    Ok(())
}

pub fn drop(repo: &GitRepo, staircase: &ResolvedStaircase, step_index: usize) -> Result<()> {
    let metadata = staircase.metadata().clone();
    if step_index >= metadata.steps.len() {
        return Err(StaircaseError::Other(
            "Step index out of bounds".to_string(),
        ));
    }

    let mut new_order: Vec<usize> = (0..metadata.steps.len()).collect();
    new_order.remove(step_index);

    reorder(repo, staircase, &new_order)
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
        let oid_to_move = repo.resolve_ref(commit_to_move)?;

        let prev_cut = if from_step_index == 0 {
            repo.resolve_ref(&metadata.target)?
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

pub fn restack(repo: &GitRepo, staircase: &ResolvedStaircase) -> Result<()> {
    let mut status = crate::core::status::get_status_metadata(repo, staircase.metadata().clone())?;

    if status.is_clean {
        return Ok(());
    }

    let target_oid = repo.resolve_ref(&status.metadata.target)?;
    let mut current_base = target_oid;
    let mut metadata_changed = false;
    let mut current_rs = staircase.clone();

    let original_cuts: Vec<String> = status
        .metadata
        .steps
        .iter()
        .map(|s| s.cut.clone())
        .collect();

    for i in 0..status.steps.len() {
        let step_status = &status.steps[i];
        let (step_name, step_branch) = {
            let step = &status.metadata.steps[i];
            (step.name.clone(), step.branch.clone())
        };

        let actual_oid = match &step_status.actual_oid {
            Some(oid) => oid.clone(),
            None => {
                return Err(StaircaseError::Other(format!(
                    "Cannot restack: branch for step '{}' is missing",
                    step_name
                )));
            }
        };

        let is_stale = !repo.is_ancestor(&current_base, &actual_oid)?;
        if is_stale {
            let old_parent_cut = if i == 0 {
                repo.merge_base(&actual_oid, &current_base)?
            } else {
                original_cuts[i - 1].clone()
            };

            let branch_name = step_branch.as_ref().ok_or_else(|| {
                StaircaseError::Other(format!("Step '{}' has no branch associated", step_name))
            })?;

            match repo.run_interactive(&[
                "rebase",
                "--onto",
                &current_base,
                &old_parent_cut,
                branch_name,
            ]) {
                Ok(_) => {
                    let new_oid = repo.resolve_ref(&format!("refs/heads/{}", branch_name))?;
                    current_rs = current_rs.update_step_oid(repo, i, new_oid.clone())?;
                    status.metadata.steps[i].cut = new_oid.clone();
                    metadata_changed = true;
                    current_base = new_oid;
                }
                Err(e) => {
                    if metadata_changed {
                        current_rs.commit_metadata(repo, status.metadata)?;
                    }
                    return Err(StaircaseError::Other(format!(
                        "Rebase failed for step '{}'. Please resolve conflicts and run restack again.\nError: {}",
                        step_name, e
                    )));
                }
            }
        } else {
            current_base = actual_oid.clone();
            if status.metadata.steps[i].cut != actual_oid {
                current_rs = current_rs.update_step_oid(repo, i, actual_oid.clone())?;
                status.metadata.steps[i].cut = actual_oid;
                metadata_changed = true;
            }
        }
    }

    if metadata_changed {
        current_rs.commit_metadata(repo, status.metadata)?;
    }

    Ok(())
}

pub fn rebase(repo: &GitRepo, staircase: &ResolvedStaircase, onto: &str) -> Result<()> {
    let mut metadata = staircase.metadata().clone();
    metadata.target = onto.to_string();
    
    let updated_rs = staircase.commit_metadata(repo, metadata)?;
    restack(repo, &updated_rs)
}

pub fn delete(repo: &GitRepo, id: &str, delete_branches: bool) -> Result<()> {
    let metadata = repo.read_metadata(id)?;

    if delete_branches {
        for step in &metadata.steps {
            if let Some(ref branch) = step.branch {
                let _ = repo.run(&["branch", "-D", branch]);
            }
        }
    }

    repo.delete_staircase_refs(id)?;
    Ok(())
}

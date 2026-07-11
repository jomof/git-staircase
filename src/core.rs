use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{BranchInfo, StaircaseMetadata, StaircaseStatus, Step, StepStatus};
use std::collections::HashMap;
use uuid::Uuid;

/// Discover potential staircases relative to `onto`.
pub fn discover(repo: &GitRepo, onto: &str) -> Result<Vec<StaircaseMetadata>> {
    let branches = repo.local_branches()?;
    let onto_oid = match repo.resolve_ref(onto) {
        Ok(oid) => oid,
        Err(_) => {
            // If onto ref doesn't exist, we can't discover relative to it.
            return Err(StaircaseError::Other(format!(
                "Onto ref '{}' not found",
                onto
            )));
        }
    };

    // Filter out branches merged to onto
    let mut active_branches = Vec::new();
    for b in branches {
        if b.refname == onto {
            continue;
        }
        // Check if b is ancestor of onto
        if !repo.is_ancestor(&b.oid, &onto_oid)? {
            active_branches.push(b);
        }
    }

    // Build adjacency list for ancestry: child -> parent (immediate ancestor)
    let mut parents: HashMap<String, String> = HashMap::new();
    for child in &active_branches {
        let mut best_parent: Option<&BranchInfo> = None;
        for parent in &active_branches {
            if child.refname == parent.refname {
                continue;
            }
            if repo.is_ancestor(&parent.oid, &child.oid)? {
                if let Some(current_best) = best_parent {
                    if repo.is_ancestor(&current_best.oid, &parent.oid)? {
                        best_parent = Some(parent);
                    }
                } else {
                    best_parent = Some(parent);
                }
            }
        }
        if let Some(p) = best_parent {
            parents.insert(child.refname.clone(), p.refname.clone());
        }
    }

    // Find roots (active branches that have no parent in active_branches)
    let mut roots = Vec::new();
    for b in &active_branches {
        if !parents.contains_key(&b.refname) {
            roots.push(b);
        }
    }

    // Build paths from roots
    let mut paths = Vec::new();
    for root in roots {
        let mut current_paths = Vec::new();
        find_paths(
            &root.refname,
            &parents,
            &active_branches,
            &mut Vec::new(),
            &mut current_paths,
        );
        paths.extend(current_paths);
    }

    let mut discovered = Vec::new();
    for path in paths {
        // Build steps for this path
        let mut steps = Vec::new();
        for refname in &path {
            let branch_info = active_branches
                .iter()
                .find(|b| &b.refname == refname)
                .unwrap();
            let short_name = refname.strip_prefix("refs/heads/").unwrap_or(refname);
            steps.push(Step {
                name: short_name.to_string(),
                cut: branch_info.oid.clone(),
                branch: Some(short_name.to_string()),
            });
        }

        // Determine name
        let branch_names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
        let name = common_prefix(&branch_names).unwrap_or_else(|| {
            // Fallback to tip branch name
            steps.last().unwrap().name.clone()
        });

        discovered.push(StaircaseMetadata {
            id: Uuid::new_v4().to_string(),
            name,
            target: onto.to_string(),
            steps,
        });
    }

    Ok(discovered)
}

fn find_paths(
    current: &str,
    parents: &HashMap<String, String>,
    all_branches: &[BranchInfo],
    current_path: &mut Vec<String>,
    results: &mut Vec<Vec<String>>,
) {
    current_path.push(current.to_string());

    // Find children (nodes that have 'current' as parent)
    let children: Vec<String> = parents
        .iter()
        .filter(|(_, parent)| *parent == current)
        .map(|(child, _)| child.clone())
        .collect();

    if children.is_empty() {
        results.push(current_path.clone());
    } else {
        for child in children {
            find_paths(&child, parents, all_branches, current_path, results);
        }
    }

    current_path.pop();
}

fn common_prefix(names: &[&str]) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let first = names[0];
    let mut len = first.len();
    for name in &names[1..] {
        let shared = first
            .chars()
            .zip(name.chars())
            .take_while(|(a, b)| a == b)
            .count();
        len = len.min(shared);
        if len == 0 {
            return None;
        }
    }
    let prefix: String = first.chars().take(len).collect();
    let trimmed = prefix.trim_end_matches(['/', '-', '_', '.', ' ']);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Adopt a staircase by writing its metadata and creating step refs.
pub fn adopt(repo: &GitRepo, staircase: &StaircaseMetadata) -> Result<()> {
    // Verify that the cuts exist and form a valid ancestry chain
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

    repo.write_metadata(staircase)?;

    // Create step refs
    for step in &staircase.steps {
        repo.update_step_ref(&staircase_id(staircase), &step.name, &step.cut)?;
    }

    Ok(())
}

fn staircase_id(s: &StaircaseMetadata) -> String {
    s.id.clone()
}

pub fn get_status(repo: &GitRepo, id: &str) -> Result<StaircaseStatus> {
    let metadata = repo.read_metadata(id)?;
    let mut steps = Vec::new();
    let mut is_clean = true;

    let mut actual_oids = Vec::new();

    for step in &metadata.steps {
        let actual_oid = if let Some(ref branch) = step.branch {
            repo.resolve_ref_opt(&format!("refs/heads/{}", branch))?
        } else {
            Some(step.cut.clone())
        };

        let is_modified = match &actual_oid {
            Some(oid) => oid != &step.cut,
            None => true,
        };

        if is_modified {
            is_clean = false;
        }

        actual_oids.push(actual_oid.clone());

        steps.push(StepStatus {
            name: step.name.clone(),
            expected_cut: step.cut.clone(),
            actual_oid,
            is_stale: false,
            is_modified,
        });
    }

    let target_oid = repo.resolve_ref(&metadata.target)?;

    for i in 0..steps.len() {
        let parent_oid = if i == 0 {
            Some(target_oid.clone())
        } else {
            actual_oids[i - 1].clone()
        };

        if let Some(ref actual) = actual_oids[i] {
            if let Some(ref parent) = parent_oid {
                let is_ancestor = repo.is_ancestor(parent, actual)?;
                if !is_ancestor {
                    steps[i].is_stale = true;
                    is_clean = false;
                }
            }
        }
    }

    Ok(StaircaseStatus {
        metadata,
        steps,
        is_clean,
    })
}

pub fn find_by_name(repo: &GitRepo, name: &str) -> Result<Option<StaircaseMetadata>> {
    let staircases = repo.list_staircases()?;
    let mut matches = Vec::new();
    for s in staircases {
        if s.name == name {
            matches.push(s);
        }
    }
    if matches.is_empty() {
        Ok(None)
    } else if matches.len() > 1 {
        Err(StaircaseError::Ambiguous(format!(
            "Multiple staircases named '{}'",
            name
        )))
    } else {
        Ok(Some(matches.remove(0)))
    }
}

pub fn split(
    repo: &GitRepo,
    id: &str,
    step_index: usize,
    at_commit: &str,
    new_step_name: Option<&str>,
) -> Result<()> {
    let mut status = get_status(repo, id)?;
    let metadata = &mut status.metadata;

    if step_index >= metadata.steps.len() {
        return Err(StaircaseError::InvalidStructure(format!(
            "Step index {} out of bounds",
            step_index
        )));
    }

    let at_oid = repo.resolve_ref(at_commit)?;

    let cut_oid = &metadata.steps[step_index].cut;
    let prev_cut_oid = if step_index == 0 {
        repo.resolve_ref(&metadata.target)?
    } else {
        metadata.steps[step_index - 1].cut.clone()
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
        return Err(StaircaseError::InvalidStructure(format!(
            "Cannot split at step boundaries"
        )));
    }

    let name = match new_step_name {
        Some(n) => n.to_string(),
        None => format!("{}-split", metadata.steps[step_index].name),
    };

    let new_step = Step {
        name: name.clone(),
        cut: at_oid.clone(),
        branch: None,
    };

    metadata.steps.insert(step_index, new_step);

    repo.write_metadata(metadata)?;
    repo.update_step_ref(&metadata.id, &name, &at_oid)?;

    Ok(())
}

pub fn join(repo: &GitRepo, id: &str, step_index_1: usize, step_index_2: usize) -> Result<()> {
    let mut status = get_status(repo, id)?;
    let metadata = &mut status.metadata;

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

    if high >= metadata.steps.len() {
        return Err(StaircaseError::InvalidStructure(format!(
            "Step index {} out of bounds",
            high
        )));
    }

    let removed_step = metadata.steps.remove(low);

    repo.write_metadata(metadata)?;
    repo.delete_step_ref(&metadata.id, &removed_step.name)?;

    Ok(())
}

pub fn restack(repo: &GitRepo, id: &str) -> Result<()> {
    let mut status = get_status(repo, id)?;

    if status.is_clean {
        return Ok(());
    }

    let target_oid = repo.resolve_ref(&status.metadata.target)?;
    let mut current_base = target_oid;
    let mut metadata_changed = false;

    let original_cuts: Vec<String> = status
        .metadata
        .steps
        .iter()
        .map(|s| s.cut.clone())
        .collect();

    for i in 0..status.steps.len() {
        let step_status = &status.steps[i];
        let (step_name, step_branch, step_cut) = {
            let step = &status.metadata.steps[i];
            (step.name.clone(), step.branch.clone(), step.cut.clone())
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

        if step_status.is_stale {
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
                    status.metadata.steps[i].cut = new_oid.clone();
                    repo.update_step_ref(&status.metadata.id, &step_name, &new_oid)?;
                    metadata_changed = true;
                    current_base = new_oid;
                }
                Err(e) => {
                    if metadata_changed {
                        repo.write_metadata(&status.metadata)?;
                    }
                    return Err(StaircaseError::Other(format!(
                        "Rebase failed for step '{}'. Please resolve conflicts and run restack again.\nError: {}",
                        step_name, e
                    )));
                }
            }
        } else {
            current_base = actual_oid.clone();
            if step_cut != actual_oid {
                status.metadata.steps[i].cut = actual_oid.clone();
                repo.update_step_ref(&status.metadata.id, &step_name, &actual_oid)?;
                metadata_changed = true;
            }
        }
    }

    if metadata_changed {
        repo.write_metadata(&status.metadata)?;
    }

    Ok(())
}

pub fn rebase(repo: &GitRepo, id: &str, onto: &str) -> Result<()> {
    let mut metadata = repo.read_metadata(id)?;
    metadata.target = onto.to_string();
    repo.write_metadata(&metadata)?;
    restack(repo, id)
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

#[cfg(test)]
mod tests {
    use super::common_prefix;

    #[test]
    fn test_common_prefix() {
        assert_eq!(
            common_prefix(&["feat/a", "feat/b"]).as_deref(),
            Some("feat")
        );
        assert_eq!(
            common_prefix(&["step-1", "step-2", "step-3"]).as_deref(),
            Some("step")
        );
        assert_eq!(
            common_prefix(&["feature-x", "feature-y"]).as_deref(),
            Some("feature")
        );
        assert_eq!(common_prefix(&["alice", "bob"]), None);
        assert_eq!(common_prefix(&["/a", "/b"]), None);
        assert_eq!(common_prefix(&[]), None);
    }
}

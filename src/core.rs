use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{
    BranchInfo, Discovery, FamilyStep, IdentityKind, ResolvedStaircase, StaircaseFamily,
    StaircaseMetadata, StaircaseStatus, Step, StepStatus, VerificationResult,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Discover potential staircases relative to `onto`.
pub fn discover(repo: &GitRepo, onto: Option<&str>) -> Result<Vec<Discovery>> {
    let branches = repo.local_branches()?;

    let onto_final = match onto {
        Some(o) => o.to_string(),
        None => infer_onto(repo)?,
    };
    let onto_oid = match repo.resolve_ref(&onto_final) {
        Ok(oid) => oid,
        Err(_) => {
            return Err(StaircaseError::Other(format!(
                "Onto ref '{}' not found",
                onto_final
            )));
        }
    };

    let mut active_branches = Vec::new();
    for b in branches {
        if b.refname == onto_final {
            continue;
        }
        if !repo.is_ancestor(&b.oid, &onto_oid)? {
            active_branches.push(b);
        }
    }

    let mut parents: HashMap<String, String> = HashMap::new();
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

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
            children_map
                .entry(p.refname.clone())
                .or_default()
                .push(child.refname.clone());
        }
    }

    let mut roots = Vec::new();
    for b in &active_branches {
        if !parents.contains_key(&b.refname) {
            roots.push(b.refname.clone());
        }
    }

    let mut discoveries = Vec::new();
    let mut discovered_branches = std::collections::HashSet::new();

    for root in roots {
        if discovered_branches.contains(&root) {
            continue;
        }

        let mut family_branches = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(current) = stack.pop() {
            if !family_branches.contains(&current) {
                family_branches.push(current.clone());
                if let Some(children) = children_map.get(&current) {
                    stack.extend(children.iter().cloned());
                }
            }
        }

        for branch in &family_branches {
            discovered_branches.insert(branch.clone());
        }

        let mut is_linear = true;
        for branch in &family_branches {
            if children_map.get(branch).is_some_and(|c| c.len() > 1) {
                is_linear = false;
                break;
            }
        }

        if is_linear {
            let mut steps = Vec::new();
            let mut current = Some(root);
            while let Some(curr) = current {
                let branch_info = active_branches.iter().find(|b| b.refname == curr).unwrap();
                let short_name = curr
                    .strip_prefix("refs/heads/")
                    .unwrap_or(&curr)
                    .to_string();
                steps.push(Step {
                    name: short_name.clone(),
                    cut: branch_info.oid.clone(),
                    branch: Some(short_name),
                });
                current = children_map.get(&curr).and_then(|c| c.first().cloned());
            }

            let branch_names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
            let name =
                common_prefix(&branch_names).unwrap_or_else(|| steps.last().unwrap().name.clone());

            discoveries.push(Discovery::Linear(StaircaseMetadata {
                id: Uuid::new_v4().to_string(),
                verification_policy: None,
                name,
                target: onto_final.to_string(),
                steps,
            }));
        } else {
            let mut steps = HashMap::new();
            for branch in &family_branches {
                let branch_info = active_branches
                    .iter()
                    .find(|b| b.refname == *branch)
                    .unwrap();
                let short_name = branch
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch)
                    .to_string();
                let children = children_map
                    .get(branch)
                    .map(|c| {
                        c.iter()
                            .map(|child| {
                                child
                                    .strip_prefix("refs/heads/")
                                    .unwrap_or(child)
                                    .to_string()
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                steps.insert(
                    short_name.clone(),
                    FamilyStep {
                        name: short_name,
                        cut: branch_info.oid.clone(),
                        branch: Some(
                            branch
                                .strip_prefix("refs/heads/")
                                .unwrap_or(branch)
                                .to_string(),
                        ),
                        children,
                    },
                );
            }

            let root_short = root
                .strip_prefix("refs/heads/")
                .unwrap_or(&root)
                .to_string();
            discoveries.push(Discovery::Ambiguous(StaircaseFamily {
                id: Uuid::new_v4().to_string(),
                verification_policy: None,
                name: format!("Family starting at {}", root_short),
                target: onto_final.to_string(),
                steps,
                roots: vec![root_short],
            }));
        }
    }

    Ok(discoveries)
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
    for step in &staircase.steps {
        repo.update_step_ref(&staircase.id, &step.name, &step.cut)?;
    }
    Ok(())
}

pub fn get_status(repo: &GitRepo, id: &str) -> Result<StaircaseStatus> {
    let metadata = repo.read_metadata(id)?;
    get_status_metadata(repo, metadata)
}

pub fn get_status_metadata(repo: &GitRepo, metadata: StaircaseMetadata) -> Result<StaircaseStatus> {
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

        if let (Some(actual), Some(parent)) = (&actual_oids[i], &parent_oid) {
            let is_ancestor = repo.is_ancestor(parent, actual)?;
            if !is_ancestor {
                steps[i].is_stale = true;
                is_clean = false;
            }
        }
    }

    Ok(StaircaseStatus {
        metadata,
        steps,
        is_clean,
    })
}

pub fn infer_onto(repo: &GitRepo) -> Result<String> {
    let mut inferred = None;

    if let Ok(Some(head)) = repo.current_branch() {
        let branches = repo.local_branches()?;
        if let Some(b) = branches.iter().find(|b| b.refname == head) {
            if let Some(ref u) = b.upstream {
                inferred = Some(u.clone());
            }
        }
    }

    if inferred.is_none() {
        for common in &["main", "master", "trunk", "develop"] {
            if let Ok(Some(_)) = repo.resolve_ref_opt(common) {
                inferred = Some(common.to_string());
                break;
            }
        }
    }

    inferred.ok_or_else(|| {
        StaircaseError::Other(
            "Could not infer integration boundary and none was provided. Use --onto to specify one."
                .to_string(),
        )
    })
}

pub fn resolve_staircase(
    repo: &GitRepo,
    name: &str,
    onto: Option<&str>,
) -> Result<Option<ResolvedStaircase>> {
    let staircases = repo.list_staircases()?;
    let mut managed_matches = Vec::new();
    for s in staircases {
        if s.name == name {
            managed_matches.push(s);
        }
    }

    let onto_final = match onto {
        Some(o) => o.to_string(),
        None => infer_onto(repo)?,
    };

    let discoveries = discover(repo, Some(&onto_final))?;
    let mut implicit_matches = Vec::new();
    for d in discoveries {
        match d {
            Discovery::Linear(s) => {
                if s.name == name {
                    // Check if this implicit match is already covered by a managed match.
                    // We consider it covered if there's a managed staircase with the same name
                    // that shares at least one step (by name and cut).
                    let is_duplicate = managed_matches.iter().any(|m| {
                        m.steps.iter().any(|m_step| {
                            s.steps.iter().any(|s_step| {
                                m_step.name == s_step.name && m_step.cut == s_step.cut
                            })
                        })
                    });
                    if !is_duplicate {
                        implicit_matches.push(s);
                    }
                }
            }
            _ => {}
        }
    }

    let total_matches = managed_matches.len() + implicit_matches.len();

    if total_matches > 1 {
        let mut msg = format!("staircase name '{}' is ambiguous", name);
        msg.push_str("\n\ncandidates:");
        for m in &managed_matches {
            msg.push_str(&format!("\n  {} (managed)", m.name));
        }
        for m in &implicit_matches {
            let desc = if let Some(step) = m.steps.last() {
                format!(
                    "{} -> {}",
                    m.steps.first().map(|s| s.name.as_str()).unwrap_or("?"),
                    step.name
                )
            } else {
                "empty".to_string()
            };
            msg.push_str(&format!("\n  {} (implicit)  {}", m.name, desc));
        }
        return Err(StaircaseError::Ambiguous(msg));
    }

    if let Some(s) = managed_matches.pop() {
        return Ok(Some(ResolvedStaircase::Managed(s)));
    }

    if let Some(s) = implicit_matches.pop() {
        return Ok(Some(ResolvedStaircase::Implicit(s)));
    }

    Ok(None)
}

pub fn find_by_name(repo: &GitRepo, name: &str) -> Result<Option<StaircaseMetadata>> {
    Ok(resolve_staircase(repo, name, None)?.map(|r| r.metadata().clone()))
}

pub fn split(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index: usize,
    at_commit: &str,
    new_step_name: Option<&str>,
) -> Result<()> {
    let mut metadata = staircase.metadata().clone();

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
        return Err(StaircaseError::InvalidStructure(
            "Cannot split at step boundaries".to_string(),
        ));
    }

    let name = match new_step_name {
        Some(n) => n.to_string(),
        None => format!("{}-split", metadata.steps[step_index].name),
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

    metadata.steps.insert(step_index, new_step);

    if staircase.is_managed() {
        repo.write_metadata(&metadata)?;
        repo.update_step_ref(&metadata.id, &name, &at_oid)?;
    } else {
        if let Some(branch_name) = new_step_name {
            repo.update_branch(branch_name, &at_oid)?;
        } else {
            adopt(repo, &metadata)?;
        }
    }

    Ok(())
}
pub fn join(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    step_index_1: usize,
    step_index_2: usize,
) -> Result<()> {
    let mut metadata = staircase.metadata().clone();

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

    if staircase.is_managed() {
        repo.write_metadata(&metadata)?;
        repo.delete_step_ref(&metadata.id, &removed_step.name)?;
    } else {
        if removed_step.branch.is_some() {
            adopt(repo, &metadata)?;
        }
    }

    Ok(())
}

pub fn restack(repo: &GitRepo, staircase: &ResolvedStaircase) -> Result<()> {
    let mut status = get_status_metadata(repo, staircase.metadata().clone())?;

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
        let (step_name, step_branch, _step_cut) = {
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
                    if staircase.is_managed() {
                        repo.update_step_ref(&status.metadata.id, &step_name, &new_oid)?;
                    }
                    metadata_changed = true;
                    current_base = new_oid;
                }
                Err(e) => {
                    if metadata_changed && staircase.is_managed() {
                        repo.write_metadata(&status.metadata)?;
                    }
                    return Err(StaircaseError::Other(format!(
                        "Rebase failed for step '{}'. Please resolve conflicts and run restack again.
Error: {}",
                        step_name, e
                    )));
                }
            }
        } else {
            current_base = actual_oid.clone();
            if status.metadata.steps[i].cut != actual_oid {
                status.metadata.steps[i].cut = actual_oid.clone();
                if staircase.is_managed() {
                    repo.update_step_ref(&status.metadata.id, &step_name, &actual_oid)?;
                }
                metadata_changed = true;
            }
        }
    }

    if metadata_changed && staircase.is_managed() {
        repo.write_metadata(&status.metadata)?;
    }

    Ok(())
}

pub fn rebase(repo: &GitRepo, staircase: &ResolvedStaircase, onto: &str) -> Result<()> {
    let mut metadata = staircase.metadata().clone();
    metadata.target = onto.to_string();
    if staircase.is_managed() {
        repo.write_metadata(&metadata)?;
    }
    let updated_rs = if staircase.is_managed() {
        ResolvedStaircase::Managed(metadata)
    } else {
        ResolvedStaircase::Implicit(metadata)
    };
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

pub fn compute_identity(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    kind: IdentityKind,
) -> Result<String> {
    if kind == IdentityKind::Lineage && !staircase.is_managed() {
        adopt(repo, staircase.metadata())?;
    }
    let staircase = staircase.metadata();
    match kind {
        IdentityKind::Lineage => Ok(staircase.id.clone()),
        IdentityKind::Nominal => Ok(staircase.name.clone()),
        IdentityKind::Revision => {
            let format = repo.get_object_format()?;
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let mut data = format!("format:{}\ntarget:{}\n", format, target_oid);
            for (i, step) in staircase.steps.iter().enumerate() {
                data.push_str(&format!("step{}:{}\n", i, step.cut));
            }
            repo.hash_data(&data)
        }
        IdentityKind::Body => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let top_oid = staircase
                .steps
                .last()
                .map(|s| s.cut.as_str())
                .unwrap_or(&target_oid);
            let data = format!("target:{}\ntop:{}\n", target_oid, top_oid);
            repo.hash_data(&data)
        }
        IdentityKind::Decomposition => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let mut patches = Vec::new();
            let mut last_cut = target_oid;
            for step in &staircase.steps {
                let patch_id = repo.get_patch_id(&last_cut, &step.cut)?;
                patches.push(patch_id);
                last_cut = step.cut.clone();
            }
            repo.hash_data(&patches.join("\n---\n"))
        }
        IdentityKind::Outcome => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let target_tree = repo.get_tree_id(&target_oid)?;
            let top_oid = staircase
                .steps
                .last()
                .map(|s| s.cut.as_str())
                .unwrap_or(&target_oid);
            let top_tree = repo.get_tree_id(top_oid)?;
            let data = format!("base-tree:{}\ntop-tree:{}\n", target_tree, top_tree);
            repo.hash_data(&data)
        }
        IdentityKind::PatchSeries => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let mut patch_ids = Vec::new();
            let mut last_cut = target_oid;
            for step in &staircase.steps {
                let patch_id = repo.get_patch_id(&last_cut, &step.cut)?;
                patch_ids.push(patch_id);
                last_cut = step.cut.clone();
            }
            repo.hash_data(&patch_ids.join("\n"))
        }
        IdentityKind::Review => Ok("".to_string()),
    }
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

pub fn verify(
    onto: Option<&str>,
    repo: &GitRepo,
    name: &str,
    build_command_override: Option<String>,
    test_command_override: Option<String>,
    aggregate_only: Option<bool>,
    each_prefix: Option<bool>,
) -> Result<Vec<VerificationResult>> {
    let rs = resolve_staircase(repo, name, onto)?
        .ok_or_else(|| StaircaseError::Other(format!("Staircase '{}' not found", name)))?;

    let s = rs.metadata();

    let policy = s.verification_policy.as_ref();

    let build_cmd = build_command_override.or(policy.and_then(|p| p.build_command.clone()));
    let test_cmd = test_command_override.or(policy.and_then(|p| p.test_command.clone()));

    let verify_each = each_prefix.unwrap_or_else(|| {
        if aggregate_only.unwrap_or(false) {
            false
        } else {
            policy.map(|p| p.verify_each_prefix).unwrap_or(false)
        }
    });

    let mut results = Vec::new();

    let mut targets = Vec::new();
    if verify_each {
        for step in &s.steps {
            targets.push((step.name.clone(), step.cut.clone()));
        }
    } else {
        if let Some(last_step) = s.steps.last() {
            targets.push(("Aggregate".to_string(), last_step.cut.clone()));
        }
    }

    if targets.is_empty() {
        return Err(StaircaseError::Other("No steps to verify".to_string()));
    }

    // Save current branch to restore later
    let original_branch = repo
        .run(&["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();

    for (step_name, cut) in targets {
        // Checkout the cut
        repo.run(&["checkout", &cut])?;

        let mut success = true;
        let mut stdout = String::new();
        let mut stderr = String::new();

        if let Some(ref cmd) = build_cmd {
            let (ok, out, err) = run_shell_command(&repo.workdir, cmd)?;
            stdout.push_str(&out);
            stderr.push_str(&err);
            if !ok {
                success = false;
            }
        }

        if success {
            if let Some(ref cmd) = test_cmd {
                let (ok, out, err) = run_shell_command(&repo.workdir, cmd)?;
                stdout.push_str(&out);
                stderr.push_str(&err);
                if !ok {
                    success = false;
                }
            }
        }

        results.push(VerificationResult {
            step_name,
            cut,
            success,
            stdout,
            stderr,
        });

        if !success {
            break;
        }
    }

    // Restore original branch
    let _ = repo.run(&["checkout", &original_branch]);

    // Record results
    let (key, kind) = if rs.is_managed() {
        (s.id.clone(), IdentityKind::Lineage)
    } else {
        (
            compute_identity(repo, &rs, IdentityKind::Revision)?,
            IdentityKind::Revision,
        )
    };
    repo.record_verification(&key, kind, &results)?;

    Ok(results)
}

fn run_shell_command(dir: &std::path::Path, command: &str) -> Result<(bool, String, String)> {
    let output = std::process::Command::new("sh")
        .current_dir(dir)
        .arg("-c")
        .arg(command)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok((output.status.success(), stdout, stderr))
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use crate::git::GitRepo;
    use crate::model::{IdentityKind, StaircaseMetadata, Step};
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn run_git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed. Stderr: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
        let path = dir.join(file);
        fs::write(path, contents).unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "--allow-empty", "-m", msg]);
        run_git(dir, &["rev-parse", "HEAD"])
    }

    fn setup_repo() -> (TempDir, GitRepo, String) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        run_git(&path, &["init", "-b", "main"]);
        let target = commit(&path, "init.txt", "initial", "initial commit");
        (tmp, GitRepo::new(path), target)
    }

    #[test]
    fn test_identity_lineage_and_nominal() {
        let (_tmp, repo, target) = setup_repo();
        let staircase = StaircaseMetadata {
            id: "test-uuid".to_string(),
            name: "test-name".to_string(),
            target: target,
            steps: vec![],
            verification_policy: None,
        };

        assert_eq!(
            compute_identity(
                &repo,
                &ResolvedStaircase::Managed(staircase.clone()),
                IdentityKind::Lineage
            )
            .unwrap(),
            "test-uuid"
        );
        assert_eq!(
            compute_identity(
                &repo,
                &ResolvedStaircase::Managed(staircase.clone()),
                IdentityKind::Nominal
            )
            .unwrap(),
            "test-name"
        );
    }

    #[test]
    fn test_identity_revision() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![Step {
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: None,
            }],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Revision,
        )
        .unwrap();
        println!(
            "ID1 PATCHES: {:?}",
            repo.run(&[
                "diff-tree",
                "-p",
                "-r",
                "--no-commit-id",
                &target.clone(),
                &c1.clone()
            ])
            .unwrap()
        );

        // Change step cut, should change revision ID
        let c2 = commit(dir, "f2.txt", "2", "c2");
        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target,
            verification_policy: None,
            steps: vec![Step {
                name: "s1".to_string(),
                cut: c2.clone(),
                branch: None,
            }],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Revision,
        )
        .unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_identity_body() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");
        let c2 = commit(dir, "f2.txt", "2", "c2");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2.clone(),
                    branch: None,
                },
            ],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Body,
        )
        .unwrap();
        println!(
            "ID1 PATCHES: {:?}",
            repo.run(&[
                "diff-tree",
                "-p",
                "-r",
                "--no-commit-id",
                &target.clone(),
                &c1.clone()
            ])
            .unwrap()
        );

        // Join steps, body ID should stay the same
        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target,
            verification_policy: None,
            steps: vec![Step {
                name: "s1+2".to_string(),
                cut: c2.clone(),
                branch: None,
            }],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Body,
        )
        .unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_identity_decomposition() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");
        let c2 = commit(dir, "f2.txt", "2", "c2");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2.clone(),
                    branch: None,
                },
            ],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Decomposition,
        )
        .unwrap();
        println!(
            "ID1 PATCHES: {:?}",
            repo.run(&[
                "diff-tree",
                "-p",
                "-r",
                "--no-commit-id",
                &target.clone(),
                &c1.clone()
            ])
            .unwrap()
        );

        // Rebase by committing same content with different messages on same target
        run_git(dir, &["checkout", &target]);
        let c1_new = commit(dir, "f1.txt", "1", "c1 rebased");
        let c2_new = commit(dir, "f2.txt", "2", "c2 rebased");

        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1_new.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2_new.clone(),
                    branch: None,
                },
            ],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Decomposition,
        )
        .unwrap();
        assert_eq!(id1, id2);

        // Squash steps, decomposition ID should change
        let s3 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![Step {
                name: "s1+2".to_string(),
                cut: c2_new.clone(),
                branch: None,
            }],
        };
        let id3 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s3.clone()),
            IdentityKind::Decomposition,
        )
        .unwrap();
        assert_ne!(id2, id3);
    }

    #[test]
    fn test_identity_outcome() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");
        let c2 = commit(dir, "f2.txt", "2", "c2");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2.clone(),
                    branch: None,
                },
            ],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Outcome,
        )
        .unwrap();
        println!(
            "ID1 PATCHES: {:?}",
            repo.run(&[
                "diff-tree",
                "-p",
                "-r",
                "--no-commit-id",
                &target.clone(),
                &c1.clone()
            ])
            .unwrap()
        );

        // Reorder commits to produce same final tree, outcome ID should stay the same
        run_git(dir, &["checkout", "main"]);
        // Start from initial commit again
        run_git(dir, &["checkout", &target]);
        commit(dir, "f2.txt", "2", "c2 reordered");
        commit(dir, "f1.txt", "1", "c1 reordered");
        let top_new = run_git(dir, &["rev-parse", "HEAD"]);

        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target,
            verification_policy: None,
            steps: vec![Step {
                name: "reordered".to_string(),
                cut: top_new,
                branch: None,
            }],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Outcome,
        )
        .unwrap();
        assert_eq!(id1, id2);
    }
}

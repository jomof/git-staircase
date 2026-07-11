use super::ResolvedStaircase;
use super::persistence;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{BranchInfo, Discovery, FamilyStep, StaircaseFamily, StaircaseMetadata, Step};
use std::collections::HashMap;
use uuid::Uuid;

use super::inference::{extract_path_to, infer_onto};
use super::utils::common_prefix;

pub fn compute_implicit_id(object_format: &str, target_oid: &str, steps: &[Step]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    // Canonical representation per spec addendum naming Section 3.11
    hasher.update(b"1"); // Discovery schema version
    hasher.update(object_format.as_bytes());
    hasher.update(target_oid.as_bytes());
    hasher.update(&(steps.len() as u32).to_be_bytes());
    for step in steps {
        hasher.update(step.cut.as_bytes());
        hasher.update(step.name.as_bytes());
    }
    let hash = hasher.finalize();
    // Truncate to 16 hex characters (8 bytes) for brevity
    format!(
        "implicit@{:016x}",
        u64::from_be_bytes(hash[..8].try_into().unwrap())
    )
}

/// Discover potential staircases relative to `onto`.
pub fn discover(repo: &GitRepo, onto: Option<&str>) -> Result<Vec<Discovery>> {
    let branches = repo.local_branches()?;

    let onto_final = match onto {
        Some(o) => o.to_string(),
        None => infer_onto(repo)?,
    };
    let onto_oid = match repo.resolve_commit(&onto_final) {
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
            if parent.oid != child.oid && repo.is_ancestor(&parent.oid, &child.oid)? {
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
                id: compute_implicit_id(&repo.get_object_format()?, &onto_oid, &steps),
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

pub fn resolve_staircase(
    repo: &GitRepo,
    name: &str,
    onto: Option<&str>,
) -> Result<Option<ResolvedStaircase>> {
    let mut resolved_staircases: HashMap<String, ResolvedStaircase> = HashMap::new();
    let mut resolved_commits: HashMap<String, String> = HashMap::new();

    // Interpretation 1: Managed
    let staircases = persistence::list_staircases(repo)?;
    for s in staircases {
        if s.name == name || s.id == name {
            resolved_staircases.insert(s.id.clone(), ResolvedStaircase::Managed(s));
        }
    }

    let onto_final = match onto {
        Some(o) => o.to_string(),
        None => infer_onto(repo)?,
    };
    let onto_oid = repo.resolve_commit(&onto_final)?;
    let object_format = repo.get_object_format()?;
    let discoveries = discover(repo, Some(&onto_final))?;

    // Interpretation 3: Implicit Name
    for d in &discoveries {
        match d {
            Discovery::Linear(s) => {
                if s.name == name || s.id == name {
                    // Prefer Managed if already present with same ID
                    if !resolved_staircases.contains_key(&s.id) {
                        resolved_staircases
                            .insert(s.id.clone(), ResolvedStaircase::Implicit(s.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    // Interpretation 2: Standard Git Revision
    if let Ok(oid) = repo.resolve_commit(name) {
        let full_name = repo
            .run(&["rev-parse", "--symbolic-full-name", name])
            .unwrap_or_else(|_| name.to_string())
            .trim()
            .to_string();
        let full_name = if full_name.is_empty() {
            name.to_string()
        } else {
            full_name
        };

        let mut matched_staircase = false;
        for d in &discoveries {
            match d {
                Discovery::Linear(s) => {
                    if let Some(pos) = s.steps.iter().position(|step| step.cut == oid) {
                        let mut sub_s = s.clone();
                        sub_s.steps.truncate(pos + 1);
                        let id = compute_implicit_id(&object_format, &onto_oid, &sub_s.steps);
                        // Prefer Managed if already present
                        if !resolved_staircases.contains_key(&id) {
                            resolved_staircases.insert(id, ResolvedStaircase::Implicit(sub_s));
                        }
                        matched_staircase = true;
                    }
                }
                Discovery::Ambiguous(f) => {
                    if let Some(step_name) =
                        f.steps.values().find(|s| s.cut == oid).map(|s| &s.name)
                    {
                        if let Some(path) = extract_path_to(f, step_name) {
                            let id = compute_implicit_id(&object_format, &onto_oid, &path.steps);
                            if !resolved_staircases.contains_key(&id) {
                                resolved_staircases.insert(id, ResolvedStaircase::Implicit(path));
                            }
                            matched_staircase = true;
                        }
                    }
                }
            }
        }
        if !matched_staircase {
            resolved_commits.insert(oid, full_name);
        }
    }

    // De-duplicate Implicit that are already Managed by content
    let managed_step_signatures: Vec<Vec<(String, String)>> = resolved_staircases
        .values()
        .filter_map(|rs| {
            if let ResolvedStaircase::Managed(m) = rs {
                Some(
                    m.steps
                        .iter()
                        .map(|step| (step.name.clone(), step.cut.clone()))
                        .collect(),
                )
            } else {
                None
            }
        })
        .collect();

    resolved_staircases.retain(|_, rs| {
        if let ResolvedStaircase::Implicit(s) = rs {
            let sig: Vec<(String, String)> = s
                .steps
                .iter()
                .map(|step| (step.name.clone(), step.cut.clone()))
                .collect();
            !managed_step_signatures.contains(&sig)
        } else {
            true
        }
    });

    let total_entities = resolved_staircases.len() + resolved_commits.len();
    if total_entities > 1 {
        let mut msg = format!("error: selector '{}' is ambiguous", name);
        let mut managed = Vec::new();
        let mut implicit = Vec::new();
        for rs in resolved_staircases.values() {
            match rs {
                ResolvedStaircase::Managed(s) => managed.push(s),
                ResolvedStaircase::Implicit(s) => implicit.push(s),
                ResolvedStaircase::ImplicitFamily(_) => {}
            }
        }
        if !managed.is_empty() {
            msg.push_str("\n\nmanaged staircase:");
            for m in managed {
                msg.push_str(&format!("\n  refs/staircases/{}", m.name));
                msg.push_str(&format!("\n  lineage: {}", m.id));
            }
        }
        if !resolved_commits.is_empty() {
            msg.push_str("\n\nGit revision:");
            for (oid, full_name) in &resolved_commits {
                msg.push_str(&format!("\n  {}", full_name));
                msg.push_str(&format!("\n  commit: {}", oid));
            }
        }
        if !implicit.is_empty() {
            msg.push_str("\n\nimplicit staircase:");
            for m in implicit {
                msg.push_str(&format!("\n  {} (implicit)", m.id));
                if let Some(step) = m.steps.last() {
                    msg.push_str(&format!("\n  top: {}", step.cut));
                }
            }
        }
        msg.push_str("\n\nuse one of:");
        msg.push_str(&format!("\n  git staircase show --name {}", name));
        if let Some(full_name) = resolved_commits.values().next() {
            msg.push_str(&format!("\n  git staircase discover --top {}", full_name));
        }
        if let Some(rs) = resolved_staircases.values().next() {
            msg.push_str(&format!(
                "\n  git staircase show --structural-key {}",
                rs.metadata().id
            ));
        }
        return Err(StaircaseError::Ambiguous(msg));
    }

    Ok(resolved_staircases.into_values().next())
}

pub fn find_by_name(repo: &GitRepo, name: &str) -> Result<Option<StaircaseMetadata>> {
    Ok(resolve_staircase(repo, name, None)?.map(|r| r.metadata().clone()))
}

pub fn resolve_explicit_staircase(
    repo: &GitRepo,
    steps: &[String],
    onto: Option<&str>,
) -> Result<ResolvedStaircase> {
    let onto_final = match onto {
        Some(o) => o.to_string(),
        None => infer_onto(repo)?,
    };
    let onto_oid = repo.resolve_commit(&onto_final)?;
    let object_format = repo.get_object_format()?;
    let mut staircase_steps = Vec::new();
    for s in steps {
        let oid = repo.resolve_commit(s)?;
        let short_name = s.strip_prefix("refs/heads/").unwrap_or(s).to_string();
        staircase_steps.push(Step {
            name: short_name.clone(),
            cut: oid,
            branch: Some(short_name),
        });
    }
    let id = compute_implicit_id(&object_format, &onto_oid, &staircase_steps);
    Ok(ResolvedStaircase::Implicit(StaircaseMetadata {
        id,
        name: steps
            .last()
            .map(|s| s.strip_prefix("refs/heads/").unwrap_or(s).to_string())
            .unwrap_or_else(|| "explicit".to_string()),
        target: onto_final,
        steps: staircase_steps,
        verification_policy: None,
    }))
}

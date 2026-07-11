use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{
    BranchInfo, Discovery, FamilyStep, StaircaseFamily, StaircaseMetadata, Step,
};
use super::ResolvedStaircase;
use std::collections::HashMap;
use uuid::Uuid;

use super::inference::{extract_path_to, infer_onto};
use super::utils::common_prefix;

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

pub fn resolve_staircase(
    repo: &GitRepo,
    name: &str,
    onto: Option<&str>,
) -> Result<Option<ResolvedStaircase>> {
    let staircases = repo.list_staircases()?;
    let mut managed_matches = Vec::new();
    for s in staircases {
        if s.name == name || s.id == name {
            managed_matches.push(s);
        }
    }

    let onto_final = match onto {
        Some(o) => o.to_string(),
        None => infer_onto(repo)?,
    };

    let discoveries = discover(repo, Some(&onto_final))?;
    let mut implicit_matches = Vec::new();

    // Try matching by nominal name
    for d in &discoveries {
        if let Discovery::Linear(s) = d {
            if s.name == name {
                implicit_matches.push(s.clone());
            }
        }
    }

    if let Ok(oid) = repo.resolve_ref(name) {
        for d in &discoveries {
            match d {
                Discovery::Linear(s) => {
                    if let Some(pos) = s.steps.iter().position(|step| step.cut == oid) {
                        let mut sub_s = s.clone();
                        sub_s.steps.truncate(pos + 1);
                        if !implicit_matches.iter().any(|m| m.steps == sub_s.steps) {
                            implicit_matches.push(sub_s);
                        }
                    }
                }
                Discovery::Ambiguous(f) => {
                    if let Some(step_name) =
                        f.steps.values().find(|s| s.cut == oid).map(|s| &s.name)
                    {
                        if let Some(path) = extract_path_to(f, step_name) {
                            if !implicit_matches.iter().any(|m| m.steps == path.steps) {
                                implicit_matches.push(path);
                            }
                        }
                    }
                }
            }
        }
    }

    // De-duplicate implicit matches that are already managed
    implicit_matches.retain(|s| {
        !managed_matches.iter().any(|m| {
            m.steps.iter().any(|m_step| {
                s.steps
                    .iter()
                    .any(|s_step| m_step.name == s_step.name && m_step.cut == s_step.cut)
            })
        })
    });

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

pub fn resolve_explicit_staircase(
    repo: &GitRepo,
    steps: &[String],
    onto: Option<&str>,
) -> Result<ResolvedStaircase> {
    let onto_final = match onto {
        Some(o) => o.to_string(),
        None => infer_onto(repo)?,
    };

    let mut staircase_steps = Vec::new();
    for s in steps {
        let oid = repo.resolve_ref(s)?;
        let short_name = s.strip_prefix("refs/heads/").unwrap_or(s).to_string();
        staircase_steps.push(Step {
            name: short_name.clone(),
            cut: oid,
            branch: Some(short_name),
        });
    }

    Ok(ResolvedStaircase::Implicit(StaircaseMetadata {
        id: Uuid::new_v4().to_string(),
        name: steps
            .last()
            .map(|s| s.strip_prefix("refs/heads/").unwrap_or(s).to_string())
            .unwrap_or_else(|| "explicit".to_string()),
        target: onto_final,
        steps: staircase_steps,
        verification_policy: None,
    }))
}

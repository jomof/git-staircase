use super::graph;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{Discovery, FamilyStep, StaircaseFamily, StaircaseMetadata, Step};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

use super::inference::infer_onto;
use super::utils::{check_sequential_layout, common_prefix};

fn hash_field(hasher: &mut sha2::Sha256, label: &[u8], value: &[u8]) {
    use sha2::Digest;
    hasher.update((label.len() as u64).to_be_bytes());
    hasher.update(label);
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

pub fn compute_implicit_id(
    repo: &GitRepo,
    integration_oid: &str,
    steps: &[Step],
) -> Result<String> {
    use sha2::{Digest, Sha256};

    let repository_identity = repo.repository_identity()?;
    let object_format = repo.get_object_format()?;
    let integration_oid = repo.resolve_commit(integration_oid)?;
    let cuts = steps
        .iter()
        .map(|step| repo.resolve_commit(&step.cut))
        .collect::<Result<Vec<_>>>()?;

    let mut hasher = Sha256::new();
    hasher.update(b"git-staircase\0implicit-structural-key\0v1\0");
    hash_field(
        &mut hasher,
        b"repository-identity",
        repository_identity.as_bytes(),
    );
    hash_field(&mut hasher, b"object-format", object_format.as_bytes());
    hash_field(
        &mut hasher,
        b"integration-context",
        integration_oid.as_bytes(),
    );
    hasher.update((cuts.len() as u64).to_be_bytes());
    for cut in cuts {
        hash_field(&mut hasher, b"cut", cut.as_bytes());
    }
    Ok(format!("implicit@{:x}", hasher.finalize()))
}

/// Discover potential staircases relative to `onto`.
pub fn discover(
    repo: &GitRepo,
    onto: Option<&str>,
    refs: Option<&str>,
    families: bool,
) -> Result<Vec<Discovery>> {
    let branches = repo.local_branches(refs)?;

    let onto_final = match onto {
        Some(o) => repo
            .resolve_symbolic_full_name(o)
            .unwrap_or_else(|_| o.to_string()),
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
    let all_oids: Vec<&str> = branches.iter().map(|b| b.oid.as_str()).collect();
    let _ = repo.preload_ancestry_ext(&all_oids, &[&onto_oid]);

    let mut active_branches = filter_active_branches(repo, branches, &onto_oid, &onto_final)?;
    active_branches.sort_by(|left, right| left.refname.cmp(&right.refname));

    let (parents, children_map) = graph::build_branch_graph(repo, &active_branches)?;

    let roots = graph::find_roots(&active_branches, &parents);

    let mut discoveries = Vec::new();
    let mut discovered_branches = std::collections::HashSet::new();

    for root in roots {
        if discovered_branches.contains(&root) {
            continue;
        }

        let family_branches = graph::collect_family(&root, &children_map);

        for branch in &family_branches {
            discovered_branches.insert(branch.clone());
        }

        if families {
            let family = build_ambiguous_family(
                &root,
                &family_branches,
                &children_map,
                &active_branches,
                &onto_final,
            );
            discoveries.push(Discovery::Ambiguous(family));
        } else {
            let paths = extract_all_linear_paths(&root, &children_map, &active_branches);
            for steps in paths {
                let branch_names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
                let name = common_prefix(&branch_names)
                    .unwrap_or_else(|| steps.last().unwrap().name.clone());

                let base = check_sequential_layout(&steps);
                let (layout, layout_base) = if let Some(b) = base {
                    (Some("sequential-v1".to_string()), Some(b))
                } else {
                    (None, None)
                };

                discoveries.push(Discovery::Linear(StaircaseMetadata {
                    landing_policy: None,
                    id: compute_implicit_id(repo, &onto_oid, &steps)?,
                    verification_policy: None,
                    name,
                    symbolic_integration_target: onto_final.to_string(),
                    steps,
                    primary_branch_layout: layout,
                    branch_layout_base: layout_base,
                    user_metadata: None,
                    lifecycle: None,
                }));
            }
        }
    }

    let mut canonical = BTreeMap::<String, Discovery>::new();
    for discovery in discoveries {
        match discovery {
            Discovery::Linear(candidate) => {
                canonical
                    .entry(candidate.id.clone())
                    .and_modify(|existing| {
                        if let Discovery::Linear(current) = existing {
                            if candidate.name < current.name {
                                current.name = candidate.name.clone();
                            }
                        }
                    })
                    .or_insert(Discovery::Linear(candidate));
            }
            Discovery::Ambiguous(family) => {
                canonical.insert(
                    format!("family:{}", family.name),
                    Discovery::Ambiguous(family),
                );
            }
        }
    }
    Ok(canonical.into_values().collect())
}

fn filter_active_branches(
    repo: &GitRepo,
    branches: Vec<crate::model::BranchInfo>,
    onto_oid: &str,
    onto_final: &str,
) -> Result<Vec<crate::model::BranchInfo>> {
    let mut active_branches = Vec::new();
    for b in branches {
        if b.refname == onto_final {
            continue;
        }
        if !repo.is_ancestor(&b.oid, onto_oid)? {
            active_branches.push(b);
        }
    }
    Ok(active_branches)
}

fn build_ambiguous_family(
    root: &str,
    family_branches: &[String],
    children_map: &HashMap<String, Vec<String>>,
    active_branches: &[crate::model::BranchInfo],
    onto_final: &str,
) -> StaircaseFamily {
    let mut steps = HashMap::new();
    for branch in family_branches {
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

    let root_short = root.strip_prefix("refs/heads/").unwrap_or(root).to_string();
    StaircaseFamily {
        id: Uuid::new_v4().to_string(),
        verification_policy: None,
        name: format!("Family starting at {}", root_short),
        symbolic_integration_target: onto_final.to_string(),
        steps,
        roots: vec![root_short],
    }
}

fn extract_all_linear_paths(
    root: &str,
    children_map: &HashMap<String, Vec<String>>,
    active_branches: &[crate::model::BranchInfo],
) -> Vec<Vec<Step>> {
    let mut paths = Vec::new();
    let mut current_path = Vec::new();
    find_paths_recursive(
        root,
        children_map,
        active_branches,
        &mut current_path,
        &mut paths,
    );
    paths
}

fn find_paths_recursive(
    curr: &str,
    children_map: &HashMap<String, Vec<String>>,
    active_branches: &[crate::model::BranchInfo],
    current_path: &mut Vec<Step>,
    paths: &mut Vec<Vec<Step>>,
) {
    let branch_info = active_branches.iter().find(|b| b.refname == curr).unwrap();
    let short_name = curr
        .strip_prefix("refs/heads/")
        .unwrap_or(&curr)
        .to_string();
    current_path.push(Step {
        id: String::new(),
        name: short_name.clone(),
        cut: branch_info.oid.clone(),
        branch: Some(short_name),
    });

    match children_map.get(curr) {
        Some(children) if !children.is_empty() => {
            for child in children {
                find_paths_recursive(child, children_map, active_branches, current_path, paths);
            }
        }
        _ => {
            paths.push(current_path.clone());
        }
    }
    current_path.pop();
}

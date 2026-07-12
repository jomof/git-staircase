use super::discovery::{compute_implicit_id, discover};
use super::persistence;
use super::{ResolvedSelector, ResolvedStaircase};
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{Discovery, StaircaseMetadata, Step};
use std::collections::HashMap;

use super::inference::{extract_path_to, infer_onto};

pub fn resolve_staircase_internal(
    repo: &GitRepo,
    name: &str,
    onto: Option<&str>,
) -> Result<Option<ResolvedStaircase>> {
    let mut resolved_staircases: HashMap<String, ResolvedStaircase> = HashMap::new();
    let mut resolved_commits: HashMap<String, String> = HashMap::new();

    // Interpretation 1: Managed
    resolve_managed(repo, name, &mut resolved_staircases)?;

    let onto_final = match onto {
        Some(o) => repo.resolve_symbolic_full_name(o).unwrap_or_else(|_| o.to_string()),
        None => infer_onto(repo)?,
    };
    let onto_oid = repo.resolve_commit(&onto_final)?;
    let object_format = repo.get_object_format()?;
    let discoveries = discover(repo, Some(&onto_final))?;

    // Interpretation 3: Implicit Name
    resolve_implicit_name(name, &discoveries, &mut resolved_staircases);

    // Interpretation 2: Standard Git Revision
    resolve_git_revision(
        repo,
        name,
        &discoveries,
        &onto_oid,
        &object_format,
        &mut resolved_staircases,
        &mut resolved_commits,
    )?;

    deduplicate_resolved(&mut resolved_staircases);

    let total_entities = resolved_staircases.len() + resolved_commits.len();
    if total_entities > 1 {
        return Err(StaircaseError::Ambiguous(report_ambiguity(
            name,
            &resolved_staircases,
            &resolved_commits,
        )));
    }

    Ok(resolved_staircases.into_values().next())
}

fn resolve_managed(
    repo: &GitRepo,
    name: &str,
    resolved_staircases: &mut HashMap<String, ResolvedStaircase>,
) -> Result<()> {
    let managed = persistence::list_staircases(repo)?;
    // Exact name or ID match
    for s in &managed {
        if s.name == name || s.id == name {
            resolved_staircases.insert(s.id.clone(), ResolvedStaircase::Managed(s.clone()));
        }
    }

    // Component match (step branch name or OID)
    if let Ok(oid) = repo.resolve_commit(name) {
        for s in managed {
            if s.steps.iter().any(|step| step.cut == oid) {
                if !resolved_staircases.contains_key(&s.id) {
                    resolved_staircases.insert(s.id.clone(), ResolvedStaircase::Managed(s));
                }
            }
        }
    }
    Ok(())
}

fn resolve_implicit_name(
    name: &str,
    discoveries: &[Discovery],
    resolved_staircases: &mut HashMap<String, ResolvedStaircase>,
) {
    for d in discoveries {
        if let Discovery::Linear(s) = d {
            if s.name == name || s.id == name {
                if !resolved_staircases.contains_key(&s.id) {
                    resolved_staircases
                        .insert(s.id.clone(), ResolvedStaircase::Implicit(s.clone()));
                }
            }
        }
    }
}

fn resolve_git_revision(
    repo: &GitRepo,
    name: &str,
    discoveries: &[Discovery],
    onto_oid: &str,
    object_format: &str,
    resolved_staircases: &mut HashMap<String, ResolvedStaircase>,
    resolved_commits: &mut HashMap<String, String>,
) -> Result<()> {
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

        // Check if already matched by managed staircase
        for rs in resolved_staircases.values() {
            if let ResolvedStaircase::Managed(m) = rs {
                if m.steps.iter().any(|step| step.cut == oid) {
                    matched_staircase = true;
                }
            }
        }

        if !matched_staircase {
            for d in discoveries {
                match d {
                    Discovery::Linear(s) => {
                        if let Some(pos) = s.steps.iter().position(|step| step.cut == oid) {
                            let mut sub_s = s.clone();
                            sub_s.steps.truncate(pos + 1);
                            let id = compute_implicit_id(object_format, onto_oid, &sub_s.steps);
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
                                let id = compute_implicit_id(object_format, onto_oid, &path.steps);
                                if !resolved_staircases.contains_key(&id) {
                                    resolved_staircases
                                        .insert(id, ResolvedStaircase::Implicit(path));
                                }
                                matched_staircase = true;
                            }
                        }
                    }
                }
            }
        }

        if !matched_staircase {
            resolved_commits.insert(oid, full_name);
        }
    }
    Ok(())
}

fn deduplicate_resolved(resolved_staircases: &mut HashMap<String, ResolvedStaircase>) {
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
}

fn report_ambiguity(
    name: &str,
    resolved_staircases: &HashMap<String, ResolvedStaircase>,
    resolved_commits: &HashMap<String, String>,
) -> String {
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
        for (oid, full_name) in resolved_commits {
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
    msg
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
        Some(o) => repo.resolve_symbolic_full_name(o).unwrap_or_else(|_| o.to_string()),
        None => infer_onto(repo)?,
    };
    let onto_oid = repo.resolve_commit(&onto_final)?;
    let object_format = repo.get_object_format()?;
    let mut staircase_steps = Vec::new();
    for s in steps {
        let oid = repo.resolve_commit(s)?;
        let short_name = s.strip_prefix("refs/heads/").unwrap_or(s).to_string();
        staircase_steps.push(Step {
            id: String::new(),
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

pub fn resolve_by_id(repo: &GitRepo, id: &str) -> Result<ResolvedStaircase> {
    let staircases = persistence::list_staircases(repo)?;
    for s in staircases {
        if s.id == id {
            return Ok(ResolvedStaircase::Managed(s));
        }
    }
    Err(StaircaseError::Other(format!(
        "Staircase with ID '{}' not found",
        id
    )))
}

pub fn resolve_by_name(repo: &GitRepo, name: &str) -> Result<ResolvedStaircase> {
    let staircases = persistence::list_staircases(repo)?;
    for s in staircases {
        if s.name == name {
            return Ok(ResolvedStaircase::Managed(s));
        }
    }
    Err(StaircaseError::Other(format!(
        "Managed staircase '{}' not found",
        name
    )))
}

pub fn resolve_by_ref(repo: &GitRepo, refname: &str) -> Result<ResolvedStaircase> {
    let oid = repo.resolve_ref(refname)?;
    resolve_by_revision(repo, &oid)
}

pub fn resolve_by_revision(repo: &GitRepo, oid: &str) -> Result<ResolvedStaircase> {
    let metadata = persistence::read_metadata_from_oid(repo, oid)?;
    // Try to find if it matches a managed staircase's current state to mark it as Managed
    let staircases = persistence::list_staircases(repo)?;
    for s in staircases {
        if s.id == metadata.id {
            // Verify if the current ref for this staircase matches this OID
            if let Ok(current_oid) = repo.resolve_ref(&format!("refs/staircases/{}", s.name)) {
                if current_oid == oid {
                    return Ok(ResolvedStaircase::Managed(metadata));
                }
            }
            // Even if it's an old revision of a managed staircase, we might want to treat it as Managed?
            // Spec says "Managed Staircase revision must resolve to a valid staircase descriptor object".
            // If it has a lineage ID that we track, it's probably best to treat it as Managed.
            return Ok(ResolvedStaircase::Managed(metadata));
        }
    }
    Ok(ResolvedStaircase::Implicit(metadata))
}

pub fn resolve_by_structural_key(
    repo: &GitRepo,
    key: &str,
    onto: Option<&str>,
) -> Result<ResolvedStaircase> {
    let onto_final = match onto {
        Some(o) => repo.resolve_symbolic_full_name(o).unwrap_or_else(|_| o.to_string()),
        None => infer_onto(repo)?,
    };
    let discoveries = discover(repo, Some(&onto_final))?;
    for d in discoveries {
        match d {
            Discovery::Linear(s) => {
                if s.id == key {
                    return Ok(ResolvedStaircase::Implicit(s));
                }
            }
            Discovery::Ambiguous(f) => {
                if f.id == key {
                    return Ok(ResolvedStaircase::ImplicitFamily(f));
                }
                // Also check paths within family
                for step_name in f.steps.keys() {
                    if let Some(path) = extract_path_to(&f, step_name) {
                        if path.id == key {
                            return Ok(ResolvedStaircase::Implicit(path));
                        }
                    }
                }
            }
        }
    }
    Err(StaircaseError::Other(format!(
        "Implicit staircase with structural key '{}' not found",
        key
    )))
}

pub fn resolve_staircase(
    repo: &GitRepo,
    name: &str,
    onto: Option<&str>,
) -> Result<Option<ResolvedSelector>> {
    if name.contains(':') {
        if let Some((sc_name, ordinal_str)) = name.rsplit_once(':') {
            if let Ok(ordinal) = ordinal_str.parse::<usize>() {
                if ordinal == 0 {
                    return Err(StaircaseError::Other(
                        "Step ordinal must be 1-based".to_string(),
                    ));
                }
                if let Some(rs) = resolve_staircase_internal(repo, sc_name, onto)? {
                    if ordinal > rs.metadata().steps.len() {
                        return Err(StaircaseError::Other(format!(
                            "Step ordinal {} out of range for staircase '{}' (which has {} steps)",
                            ordinal,
                            sc_name,
                            rs.metadata().steps.len()
                        )));
                    }
                    return Ok(Some(ResolvedSelector {
                        staircase: rs,
                        step_index: Some(ordinal - 1),
                    }));
                }
            }
        }
    }

    Ok(
        resolve_staircase_internal(repo, name, onto)?.map(|rs| ResolvedSelector {
            staircase: rs,
            step_index: None,
        }),
    )
}

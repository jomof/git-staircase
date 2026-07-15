use super::discovery::{compute_implicit_id, discover};
use super::persistence;
use super::{ResolvedSelector, ResolvedStaircase};
use crate::core::refs::{PUBLIC_PREFIX, StaircaseRefs};
use crate::error::{AmbiguityCandidate, Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{Discovery, StaircaseMetadata, Step};
use std::collections::BTreeMap;

use super::inference::{extract_path_to, infer_onto};
use super::utils::check_sequential_layout;

pub fn resolve_staircase_internal(
    repo: &GitRepo,
    name: &str,
    onto: Option<&str>,
) -> Result<Option<ResolvedStaircase>> {
    let mut resolved_staircases: BTreeMap<String, ResolvedStaircase> = BTreeMap::new();
    let mut resolved_commits: BTreeMap<String, String> = BTreeMap::new();

    // Interpretation 1: Managed
    resolve_managed(repo, name, &mut resolved_staircases)?;

    // Interpretation: Implicit Archive Snapshot
    resolve_implicit_archive(repo, name, &mut resolved_staircases)?;

    let onto_final = match onto {
        Some(o) => repo
            .resolve_symbolic_full_name(o)
            .unwrap_or_else(|_| o.to_string()),
        None => infer_onto(repo)?,
    };
    let onto_oid = repo.resolve_commit(&onto_final)?;
    let discoveries = discover(repo, Some(&onto_final), None, false)?;

    // Interpretation 3: Implicit Name
    resolve_implicit_name(name, &discoveries, &mut resolved_staircases);

    // Interpretation 2: Standard Git Revision
    resolve_git_revision(
        repo,
        name,
        &discoveries,
        &onto_oid,
        &mut resolved_staircases,
        &mut resolved_commits,
    )?;

    resolve_structural_key(name, &discoveries, &mut resolved_staircases);
    deduplicate_resolved(repo, &mut resolved_staircases)?;

    let total_entities = resolved_staircases.len() + resolved_commits.len();
    if total_entities > 1 {
        return Err(StaircaseError::SelectorAmbiguous {
            selector: name.to_string(),
            candidates: ambiguity_candidates(repo, &resolved_staircases, &resolved_commits),
        });
    }

    Ok(resolved_staircases.into_values().next())
}

fn resolve_managed(
    repo: &GitRepo,
    name: &str,
    resolved_staircases: &mut BTreeMap<String, ResolvedStaircase>,
) -> Result<()> {
    let stripped_name = name
        .strip_prefix(PUBLIC_PREFIX)
        .or_else(|| name.strip_prefix("staircases/"))
        .unwrap_or(name);

    let managed = persistence::list_all_staircases(repo)?;
    // Bare managed interpretation is an exact canonical name.
    for s in &managed {
        if s.name == stripped_name {
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

fn resolve_implicit_archive(
    repo: &GitRepo,
    name: &str,
    resolved_staircases: &mut BTreeMap<String, ResolvedStaircase>,
) -> Result<()> {
    if let Ok(snapshots) = persistence::list_implicit_archive_snapshots(repo) {
        for snap in snapshots {
            let archive_id = &snap.archive_id;
            let display_name = &snap.descriptor.canonical_display_name;
            let archive_sel = format!("archive@{}", archive_id);

            if name == archive_sel
                || name == archive_id
                || name == display_name
                || archive_id.starts_with(name)
            {
                resolved_staircases.insert(
                    format!("archive:{}", archive_id),
                    ResolvedStaircase::ImplicitArchive(snap),
                );
            }
        }
    }
    Ok(())
}

fn resolve_implicit_name(
    name: &str,
    discoveries: &[Discovery],
    resolved_staircases: &mut BTreeMap<String, ResolvedStaircase>,
) {
    if resolved_staircases.values().any(|resolved| {
        matches!(
            resolved,
            ResolvedStaircase::Managed(metadata) if metadata.name == name
        )
    }) {
        return;
    }
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
    resolved_staircases: &mut BTreeMap<String, ResolvedStaircase>,
    resolved_commits: &mut BTreeMap<String, String>,
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
                            let id = compute_implicit_id(repo, onto_oid, &sub_s.steps)?;
                            sub_s.id = id.clone();
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
                            if let Some(mut path) = extract_path_to(f, step_name) {
                                let id = compute_implicit_id(repo, onto_oid, &path.steps)?;
                                path.id = id.clone();
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

fn structural_identity(repo: &GitRepo, staircase: &StaircaseMetadata) -> Result<String> {
    let integration = repo.resolve_commit(&staircase.symbolic_integration_target)?;
    compute_implicit_id(repo, &integration, &staircase.steps)
}

fn resolve_structural_key(
    selector: &str,
    discoveries: &[Discovery],
    resolved_staircases: &mut BTreeMap<String, ResolvedStaircase>,
) {
    if !selector.starts_with("implicit@") {
        return;
    }
    for discovery in discoveries {
        if let Discovery::Linear(staircase) = discovery {
            if staircase.id.starts_with(selector) {
                resolved_staircases
                    .entry(staircase.id.clone())
                    .or_insert_with(|| ResolvedStaircase::Implicit(staircase.clone()));
            }
        }
    }
}

fn deduplicate_resolved(
    repo: &GitRepo,
    resolved_staircases: &mut BTreeMap<String, ResolvedStaircase>,
) -> Result<()> {
    let managed_keys: Vec<String> = resolved_staircases
        .values()
        .filter_map(|rs| {
            if let ResolvedStaircase::Managed(m) = rs {
                structural_identity(repo, m).ok()
            } else {
                None
            }
        })
        .collect();

    resolved_staircases.retain(|_, rs| {
        if let ResolvedStaircase::Implicit(s) = rs {
            structural_identity(repo, s)
                .map(|key| !managed_keys.contains(&key))
                .unwrap_or(true)
        } else {
            true
        }
    });
    Ok(())
}

fn ambiguity_candidates(
    repo: &GitRepo,
    resolved_staircases: &BTreeMap<String, ResolvedStaircase>,
    resolved_commits: &BTreeMap<String, String>,
) -> Vec<AmbiguityCandidate> {
    let mut candidates = Vec::new();
    for staircase in resolved_staircases.values() {
        let metadata = staircase.metadata();
        let managed = staircase.is_managed();
        candidates.push(AmbiguityCandidate {
            kind: if managed { "managed" } else { "implicit" }.into(),
            selector: if managed {
                format!("--id {}", metadata.id)
            } else {
                format!("--structural-key {}", metadata.id)
            },
            name: Some(metadata.name.clone()),
            lineage_id: managed.then(|| metadata.id.clone()),
            structural_key: (!managed).then(|| metadata.id.clone()),
            record_oid: managed
                .then(|| {
                    repo.resolve_ref(&StaircaseRefs::state_record(&metadata.id))
                        .ok()
                })
                .flatten(),
            integration_context: repo.resolve_commit(&metadata.symbolic_integration_target).ok(),
            cuts: metadata.steps.iter().map(|step| step.cut.clone()).collect(),
        });
    }
    for (oid, full_name) in resolved_commits {
        candidates.push(AmbiguityCandidate {
            kind: "git-revision".into(),
            selector: full_name.clone(),
            name: None,
            lineage_id: None,
            structural_key: None,
            record_oid: None,
            integration_context: None,
            cuts: vec![oid.clone()],
        });
    }
    candidates
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
        Some(o) => repo
            .resolve_symbolic_full_name(o)
            .unwrap_or_else(|_| o.to_string()),
        None => infer_onto(repo)?,
    };
    let onto_oid = repo.resolve_commit(&onto_final)?;
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
    let id = compute_implicit_id(repo, &onto_oid, &staircase_steps)?;
    let base = check_sequential_layout(&staircase_steps);
    let (layout, layout_base) = if let Some(b) = base {
        (Some("sequential-v1".to_string()), Some(b))
    } else {
        (None, None)
    };
    Ok(ResolvedStaircase::Implicit(StaircaseMetadata {
        landing_policy: None,
        id,
        name: steps
            .last()
            .map(|s| s.strip_prefix("refs/heads/").unwrap_or(s).to_string())
            .unwrap_or_else(|| "explicit".to_string()),
        symbolic_integration_target: onto_final,
        steps: staircase_steps,
        verification_policy: None,
        primary_branch_layout: layout,
        branch_layout_base: layout_base,
        user_metadata: None,
        lifecycle: None,
    }))
}

pub fn resolve_by_id(repo: &GitRepo, id: &str) -> Result<ResolvedStaircase> {
    let staircases = persistence::list_all_staircases(repo)?;
    for s in staircases {
        if s.id == id {
            return Ok(ResolvedStaircase::Managed(s));
        }
    }
    Err(StaircaseError::NotFound(format!("lineage ID '{}'", id)))
}

pub fn resolve_by_name(repo: &GitRepo, name: &str) -> Result<ResolvedStaircase> {
    let staircases = persistence::list_all_staircases(repo)?;
    for s in staircases {
        if s.name == name {
            return Ok(ResolvedStaircase::Managed(s));
        }
    }
    Err(StaircaseError::NotFound(format!("managed name '{}'", name)))
}

pub fn resolve_by_ref(repo: &GitRepo, refname: &str) -> Result<ResolvedStaircase> {
    if !refname.starts_with(PUBLIC_PREFIX) {
        return Err(StaircaseError::Other(format!(
            "'{}' is not a full staircase ref",
            refname
        )));
    }
    let oid = repo.resolve_ref(refname)?;
    resolve_by_record(repo, &oid)
}

pub fn resolve_by_record(repo: &GitRepo, oid: &str) -> Result<ResolvedStaircase> {
    let metadata = persistence::read_metadata_from_oid(repo, oid)?;
    // Try to find if it matches a managed staircase's current state to mark it as Managed
    let staircases = persistence::list_staircases(repo)?;
    for s in staircases {
        if s.id == metadata.id {
            // Verify if the current ref for this staircase matches this OID
            if let Ok(current_oid) = repo.resolve_ref(&StaircaseRefs::public(&s.name)) {
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

pub fn resolve_by_revision(repo: &GitRepo, oid: &str) -> Result<ResolvedStaircase> {
    resolve_by_record(repo, oid)
}

pub fn resolve_by_structural_key(
    repo: &GitRepo,
    key: &str,
    onto: Option<&str>,
) -> Result<ResolvedStaircase> {
    let onto_final = match onto {
        Some(o) => repo
            .resolve_symbolic_full_name(o)
            .unwrap_or_else(|_| o.to_string()),
        None => infer_onto(repo)?,
    };
    let discoveries = discover(repo, Some(&onto_final), None, false)?;
    let mut matches = Vec::new();
    for d in discoveries {
        match d {
            Discovery::Linear(s) => {
                if s.id.starts_with(key) {
                    matches.push(ResolvedStaircase::Implicit(s));
                }
            }
            Discovery::Ambiguous(f) => {
                if f.id.starts_with(key) {
                    matches.push(ResolvedStaircase::ImplicitFamily(f.clone()));
                }
                // Also check paths within family
                for step_name in f.steps.keys() {
                    if let Some(path) = extract_path_to(&f, step_name) {
                        if path.id.starts_with(key) {
                            matches.push(ResolvedStaircase::Implicit(path));
                        }
                    }
                }
            }
        }
    }
    matches.sort_by(|left, right| {
        let left_id = match left {
            ResolvedStaircase::ImplicitFamily(f) => &f.id,
            _ => &left.metadata().id,
        };
        let right_id = match right {
            ResolvedStaircase::ImplicitFamily(f) => &f.id,
            _ => &right.metadata().id,
        };
        left_id.cmp(right_id)
    });
    matches.dedup();
    match matches.len() {
        0 => Err(StaircaseError::NotFound(format!(
            "implicit structural key '{}'",
            key
        ))),
        1 => Ok(matches.remove(0)),
        _ => {
            let mut candidates = Vec::new();
            for staircase in matches {
                let metadata = staircase.metadata();
                candidates.push(AmbiguityCandidate {
                    kind: "implicit".into(),
                    selector: format!("--structural-key {}", metadata.id),
                    name: Some(metadata.name.clone()),
                    lineage_id: None,
                    structural_key: Some(metadata.id.clone()),
                    record_oid: None,
                    integration_context: repo.resolve_commit(&metadata.symbolic_integration_target).ok(),
                    cuts: metadata.steps.iter().map(|step| step.cut.clone()).collect(),
                });
            }
            Err(StaircaseError::SelectorAmbiguous {
                selector: key.to_string(),
                candidates,
            })
        }
    }
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

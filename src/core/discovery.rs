use super::persistence;
use super::{ResolvedSelector, ResolvedStaircase};
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

    let active_branches = filter_active_branches(repo, branches, &onto_oid, &onto_final)?;

    let (parents, children_map) = build_branch_graph(repo, &active_branches)?;

    let roots = find_roots(&active_branches, &parents);

    let mut discoveries = Vec::new();
    let mut discovered_branches = std::collections::HashSet::new();

    for root in roots {
        if discovered_branches.contains(&root) {
            continue;
        }

        let family_branches = collect_family(&root, &children_map);

        for branch in &family_branches {
            discovered_branches.insert(branch.clone());
        }

        let is_linear = family_branches
            .iter()
            .all(|branch| children_map.get(branch).map_or(true, |c| c.len() <= 1));

        if is_linear {
            let steps = extract_linear_staircase(&root, &children_map, &active_branches);
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
            let family = build_ambiguous_family(
                &root,
                &family_branches,
                &children_map,
                &active_branches,
                &onto_final,
            );
            discoveries.push(Discovery::Ambiguous(family));
        }
    }

    Ok(discoveries)
}

fn filter_active_branches(
    repo: &GitRepo,
    branches: Vec<BranchInfo>,
    onto_oid: &str,
    onto_final: &str,
) -> Result<Vec<BranchInfo>> {
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

fn build_branch_graph(
    repo: &GitRepo,
    active_branches: &[BranchInfo],
) -> Result<(HashMap<String, String>, HashMap<String, Vec<String>>)> {
    let mut parents: HashMap<String, String> = HashMap::new();
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

    for child in active_branches {
        let mut best_parent: Option<&BranchInfo> = None;
        for parent in active_branches {
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
    Ok((parents, children_map))
}

fn find_roots(active_branches: &[BranchInfo], parents: &HashMap<String, String>) -> Vec<String> {
    let mut roots = Vec::new();
    for b in active_branches {
        if !parents.contains_key(&b.refname) {
            roots.push(b.refname.clone());
        }
    }
    roots
}

fn collect_family(root: &str, children_map: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut family_branches = Vec::new();
    let mut stack = vec![root.to_string()];
    while let Some(current) = stack.pop() {
        if !family_branches.contains(&current) {
            family_branches.push(current.clone());
            if let Some(children) = children_map.get(&current) {
                stack.extend(children.iter().cloned());
            }
        }
    }
    family_branches
}

fn extract_linear_staircase(
    root: &str,
    children_map: &HashMap<String, Vec<String>>,
    active_branches: &[BranchInfo],
) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut current = Some(root.to_string());
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
    steps
}

fn build_ambiguous_family(
    root: &str,
    family_branches: &[String],
    children_map: &HashMap<String, Vec<String>>,
    active_branches: &[BranchInfo],
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
        target: onto_final.to_string(),
        steps,
        roots: vec![root_short],
    }
}

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
        Some(o) => o.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitRepo;
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
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn commit(dir: &Path, file: &str, content: &str, msg: &str) -> String {
        fs::write(dir.join(file), content).unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "-m", msg]);
        run_git(dir, &["rev-parse", "HEAD"])
    }

    #[test]
    fn test_build_branch_graph_linear() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        run_git(dir, &["init", "-b", "main"]);
        let _root = commit(dir, "root.txt", "root", "root");

        let c1 = commit(dir, "1.txt", "1", "c1");
        run_git(dir, &["branch", "b1", &c1]);

        let c2 = commit(dir, "2.txt", "2", "c2");
        run_git(dir, &["branch", "b2", &c2]);

        let repo = GitRepo::new(dir.to_path_buf());
        let active_branches = vec![
            BranchInfo {
                refname: "refs/heads/b1".to_string(),
                oid: c1.clone(),
                upstream: None,
            },
            BranchInfo {
                refname: "refs/heads/b2".to_string(),
                oid: c2.clone(),
                upstream: None,
            },
        ];

        let (parents, children) = build_branch_graph(&repo, &active_branches).unwrap();

        assert_eq!(parents.len(), 1);
        assert_eq!(parents["refs/heads/b2"], "refs/heads/b1");
        assert_eq!(children["refs/heads/b1"], vec!["refs/heads/b2"]);
    }

    #[test]
    fn test_find_roots() {
        let active_branches = vec![
            BranchInfo {
                refname: "b1".to_string(),
                oid: "1".to_string(),
                upstream: None,
            },
            BranchInfo {
                refname: "b2".to_string(),
                oid: "2".to_string(),
                upstream: None,
            },
        ];
        let mut parents = HashMap::new();
        parents.insert("b2".to_string(), "b1".to_string());

        let roots = find_roots(&active_branches, &parents);
        assert_eq!(roots, vec!["b1".to_string()]);
    }

    #[test]
    fn test_collect_family() {
        let mut children = HashMap::new();
        children.insert("b1".to_string(), vec!["b2".to_string(), "b3".to_string()]);
        children.insert("b2".to_string(), vec!["b4".to_string()]);

        let family = collect_family("b1", &children);
        assert!(family.contains(&"b1".to_string()));
        assert!(family.contains(&"b2".to_string()));
        assert!(family.contains(&"b3".to_string()));
        assert!(family.contains(&"b4".to_string()));
        assert_eq!(family.len(), 4);
    }
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
        Some(o) => o.to_string(),
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

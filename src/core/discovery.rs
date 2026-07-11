use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{
    BranchInfo, Discovery, FamilyStep, IdentityKind, ResolvedStaircase, StaircaseFamily,
    StaircaseMetadata, Step,
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

pub fn common_prefix(names: &[&str]) -> Option<String> {
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

pub fn extract_path_to(
    family: &StaircaseFamily,
    target_step_name: &str,
) -> Option<StaircaseMetadata> {
    let mut path_steps = Vec::new();
    let mut current = target_step_name.to_string();

    loop {
        let step = family.steps.get(&current)?;
        path_steps.push(Step {
            name: step.name.clone(),
            cut: step.cut.clone(),
            branch: step.branch.clone(),
        });

        // Find parent
        let parent = family
            .steps
            .values()
            .find(|s| s.children.contains(&current));
        if let Some(p) = parent {
            current = p.name.clone();
        } else {
            break;
        }
    }

    path_steps.reverse();

    Some(StaircaseMetadata {
        id: uuid::Uuid::new_v4().to_string(),
        name: target_step_name.to_string(),
        target: family.target.clone(),
        steps: path_steps,
        verification_policy: family.verification_policy.clone(),
    })
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

pub fn compute_identity(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    kind: IdentityKind,
) -> Result<String> {
    if kind == IdentityKind::Lineage && !staircase.is_managed() {
        super::manipulation::adopt(repo, staircase.metadata())?;
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

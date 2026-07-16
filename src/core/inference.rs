use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{StaircaseFamily, StaircaseMetadata, Step};
use crate::workspace::storage::find_workspace_record_for_path;

pub fn infer_onto(repo: &GitRepo) -> Result<String> {
    let mut inferred = None;

    // 1. Bound integration-context provider / workspace configuration
    if let Ok(Some(ws)) = find_workspace_record_for_path(&repo.workdir) {
        if let Some(proj_id) = ws.current_project_id {
            if let Ok(Some(oid)) = repo.resolve_commit_opt(&proj_id) {
                inferred = Some(oid);
            }
        }
    }

    // 1b. Check for repo workspace manifest evidence directly if record-based inference failed
    if inferred.is_none() {
        if let Ok(Some(report)) = crate::workspace::repo_provider::observe_repo_workspace(repo) {
            // Prefer strong/authoritative candidates that are NOT the current HEAD if we are on a branch
            let head_oid = repo.resolve_commit("HEAD").ok();
            let current_branch = repo.current_branch().ok().flatten();

            let mut best_candidate = None;
            for cand in &report.integration_candidates {
                if !cand.eligible {
                    continue;
                }
                let Some(oid) = cand.resolved_oid.clone() else {
                    continue;
                };

                // If we are on a branch, don't use the branch's own OID as the anchor
                if let Some(ref branch) = current_branch {
                    if let Ok(branch_oid) = repo.resolve_commit(branch) {
                        if oid == branch_oid {
                            continue;
                        }
                    }
                } else if let Some(ref head) = head_oid {
                    if oid == *head {
                        continue;
                    }
                }

                match cand.authority {
                    crate::workspace::model::EvidenceAuthority::Authoritative => {
                        best_candidate = Some(oid);
                        break;
                    }
                    crate::workspace::model::EvidenceAuthority::Strong => {
                        if best_candidate.is_none() {
                            best_candidate = Some(oid);
                        }
                    }
                    _ => {}
                }
            }

            if let Some(oid) = best_candidate {
                inferred = Some(oid);
            } else {
                // Fallback to any eligible candidate if no strong one found
                if let Some(cand) = report
                    .integration_candidates
                    .iter()
                    .find(|c| c.eligible && c.resolved_oid.is_some())
                {
                    inferred = cand.resolved_oid.clone();
                }
            }
        }
    }

    // 2. Applicable branch upstream configuration
    if inferred.is_none() {
        if let Ok(Some(head)) = repo.current_branch() {
            let branches = repo.local_branches(None)?;
            if let Some(b) = branches.iter().find(|b| b.refname == head) {
                if let Some(ref u) = b.upstream {
                    inferred = Some(u.clone());
                }
            }
        }
    }

    // 3. Eligible detached HEAD
    if inferred.is_none() {
        if let Ok(None) = repo.current_branch() {
            // Check no active git rebase/merge operation
            let git_dir = repo.workdir.join(".git");
            let rebase_interactive = git_dir.join("rebase-merge").exists();
            let rebase_apply = git_dir.join("rebase-apply").exists();
            let merge_head = git_dir.join("MERGE_HEAD").exists();
            let cherry_pick_head = git_dir.join("CHERRY_PICK_HEAD").exists();

            if !rebase_interactive && !rebase_apply && !merge_head && !cherry_pick_head {
                if let Ok(head_oid) = repo.resolve_commit("HEAD") {
                    inferred = Some(head_oid);
                }
            }
        }
    }

    // 4. Unique compatible remote-default evidence or common branch names
    if inferred.is_none() {
        for common in &[
            "refs/remotes/origin/main",
            "refs/remotes/origin/master",
            "main",
            "master",
            "trunk",
            "develop",
        ] {
            if let Ok(Some(_)) = repo.resolve_commit_opt(common) {
                if let Ok(full) = repo.resolve_symbolic_full_name(common) {
                    inferred = Some(full);
                    break;
                }
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

pub fn extract_path_to(
    family: &StaircaseFamily,
    target_step_name: &str,
) -> Option<StaircaseMetadata> {
    let mut path_steps = Vec::new();
    let mut current = target_step_name.to_string();

    loop {
        let step = family.steps.get(&current)?;
        path_steps.push(Step {
            id: String::new(),
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
        landing_policy: None,
        id: uuid::Uuid::new_v4().to_string(),
        name: target_step_name.to_string(),
        target: family.target.clone(),
        steps: path_steps,
        verification_policy: family.verification_policy.clone(),

        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    })
}

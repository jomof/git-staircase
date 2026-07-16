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
            } else if let Some(rev) = ws.discovery_fingerprint.get("revision") {
                if let Ok(Some(oid)) = repo.resolve_commit_opt(rev) {
                    inferred = Some(oid);
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
            "refs/remotes/m/main",
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

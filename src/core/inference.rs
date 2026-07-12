use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{StaircaseFamily, StaircaseMetadata, Step};

pub fn infer_onto(repo: &GitRepo) -> Result<String> {
    let mut inferred = None;

    if let Ok(Some(head)) = repo.current_branch() {
        let branches = repo.local_branches(None)?;
        if let Some(b) = branches.iter().find(|b| b.refname == head) {
            if let Some(ref u) = b.upstream {
                inferred = Some(u.clone());
            }
        }
    }

    if inferred.is_none() {
        for common in &["main", "master", "trunk", "develop"] {
            if let Ok(Some(_)) = repo.resolve_commit_opt(common) {
                inferred = Some(repo.resolve_symbolic_full_name(common)?);
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
        id: uuid::Uuid::new_v4().to_string(),
        name: target_step_name.to_string(),
        target: family.target.clone(),
        steps: path_steps,
        verification_policy: family.verification_policy.clone(),
    })
}

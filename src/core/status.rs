use super::persistence;
use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{IdentityKind, StaircaseMetadata, StaircaseStatus, StepStatus};

pub fn get_status(repo: &GitRepo, id: &str) -> Result<StaircaseStatus> {
    let metadata = persistence::read_metadata(repo, id)?;
    get_status_metadata(repo, metadata, false)
}

pub fn get_status_metadata(
    repo: &GitRepo,
    metadata: StaircaseMetadata,
    is_implicit: bool,
) -> Result<StaircaseStatus> {
    let mut steps = Vec::new();
    let mut is_clean = true;

    let mut actual_oids = Vec::new();

    for step in &metadata.steps {
        let actual_oid = if let Some(ref branch) = step.branch {
            repo.resolve_commit_opt(&format!("refs/heads/{}", branch))?
        } else {
            Some(step.cut.clone())
        };

        let is_modified = match &actual_oid {
            Some(oid) => oid != &step.cut,
            None => true,
        };

        if is_modified {
            is_clean = false;
        }

        actual_oids.push(actual_oid.clone());

        steps.push(StepStatus {
            name: step.name.clone(),
            expected_cut: step.cut.clone(),
            actual_oid,
            is_stale: false,
            is_modified,
        });
    }

    let target_oid = repo.resolve_commit(&metadata.target)?;

    for i in 0..steps.len() {
        let parent_oid = if i == 0 {
            Some(target_oid.clone())
        } else {
            actual_oids[i - 1].clone()
        };

        if let (Some(actual), Some(parent)) = (&actual_oids[i], &parent_oid) {
            let is_ancestor = repo.is_ancestor(parent, actual)?;
            if !is_ancestor {
                steps[i].is_stale = true;
                is_clean = false;
            }
        }
    }

    let verification_results = if is_implicit {
        None
    } else {
        persistence::read_verification(repo, &metadata.id, IdentityKind::Lineage)?
    };

    Ok(StaircaseStatus {
        metadata,
        steps,
        is_clean,
        is_implicit,
        verification_results,
    })
}

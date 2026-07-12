use super::persistence;
use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{
    Discovery, IdentityKind, StaircaseMetadata, StaircaseStatus, StepStatus, WorktreeDraft,
};

pub fn get_status(repo: &GitRepo, id: &str) -> Result<StaircaseStatus> {
    let metadata = persistence::read_metadata(repo, id)?;
    get_status_metadata(repo, metadata, false)
}

pub fn get_status_metadata(
    repo: &GitRepo,
    metadata: StaircaseMetadata,
    is_implicit: bool,
) -> Result<StaircaseStatus> {
    get_status_metadata_ext(repo, metadata, is_implicit, None, None)
}

pub fn get_status_metadata_ext(
    repo: &GitRepo,
    metadata: StaircaseMetadata,
    is_implicit: bool,
    known_discoveries: Option<&[Discovery]>,
    cached_draft: Option<Option<WorktreeDraft>>,
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

        let is_incomplete = actual_oid.is_none();
        let is_modified = match &actual_oid {
            Some(oid) => oid != &step.cut,
            None => false,
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
            is_incomplete,
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

    let mut is_diverged = false;
    let mut is_ambiguous = false;

    // Detect diverged: multiple branches matching the step name if we use conventions
    // Or simply if refs and metadata disagree and it is not a clean FF.
    for step in &steps {
        if step.is_modified {
            // For now, any disagreement is "Diverged" per spec 8.3
            is_diverged = true;
        }
    }

    // Detect ambiguous: if this managed staircase name is shared by an implicit one
    if let Some(discoveries) = known_discoveries {
        for d in discoveries {
            if let Discovery::Linear(m) = d {
                if m.name == metadata.name && m.id != metadata.id {
                    is_ambiguous = true;
                }
            }
        }
    } else if let Ok(discoveries) = super::discovery::discover(repo, Some(&metadata.target), None, false) {
        for d in discoveries {
            if let Discovery::Linear(m) = d {
                if m.name == metadata.name && m.id != metadata.id {
                    is_ambiguous = true;
                }
            }
        }
    }

    let filter_draft = |d: &WorktreeDraft| {
        if let Some(ref att) = d.attachment {
            if let Some(ref sname) = att.staircase_name {
                if sname == &metadata.name {
                    return true;
                }
            }
            if let Some(ref sid) = att.staircase_id {
                if sid == &metadata.id {
                    return true;
                }
            }
        }
        metadata.steps.iter().any(|s| s.cut == d.basis)
    };

    let worktree_draft = match cached_draft {
        Some(draft_opt) => draft_opt.filter(filter_draft),
        None => super::draft::get_worktree_draft(repo).ok().filter(filter_draft),
    };

    Ok(StaircaseStatus {
        metadata,
        steps,
        is_clean,
        is_implicit,
        is_diverged,
        is_ambiguous,
        verification_results,
        worktree_draft,
    })
}

use crate::core::utils::current_timestamp;
use crate::core::refs::StaircaseRefs;
use crate::core::persistence;
use crate::core::resolved::ResolvedSelector;
use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{StaircaseLink, StaircaseRecord, StaircaseMetadata, StaircaseUserMetadata, StepMetadata};

pub fn get_user_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
) -> Result<StaircaseUserMetadata> {
    let meta = selector.staircase.metadata();
    let record_ref = StaircaseRefs::state_record(&meta.id);
    let archive_ref = StaircaseRefs::archive_record(&meta.id);

    let record = persistence::read_record(repo, &record_ref)
        .or_else(|_| persistence::read_record(repo, &StaircaseRefs::public(&meta.name)))
        .or_else(|_| persistence::read_record(repo, &archive_ref))?;

    Ok(record.user_metadata)
}

pub fn update_user_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    mut new_user_meta: StaircaseUserMetadata,
) -> Result<StaircaseRecord> {
    let meta = selector.staircase.metadata();
    let record_ref = StaircaseRefs::state_record(&meta.id);
    let archive_ref = StaircaseRefs::archive_record(&meta.id);

    let record = persistence::read_record(repo, &record_ref)
        .or_else(|_| persistence::read_record(repo, &StaircaseRefs::public(&meta.name)))
        .or_else(|_| persistence::read_record(repo, &archive_ref))?;

    new_user_meta.labels.sort();
    new_user_meta.labels.dedup();
    new_user_meta.updated_at = Some(current_timestamp());

    let updated_record = persistence::write_record(
        repo,
        &record.metadata,
        &new_user_meta,
        &record.lifecycle,
        record.archive_manifest.as_ref(),
        true,
    )?;

    Ok(updated_record)
}

pub fn set_title(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    title: &str,
) -> Result<StaircaseRecord> {
    let mut user_meta = get_user_metadata(repo, selector)?;
    if user_meta.created_at.is_none() {
        user_meta.created_at = Some(current_timestamp());
    }
    user_meta.title = Some(title.to_string());
    update_user_metadata(repo, selector, user_meta)
}

pub fn set_description(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    description: &str,
) -> Result<StaircaseRecord> {
    let mut user_meta = get_user_metadata(repo, selector)?;
    if user_meta.created_at.is_none() {
        user_meta.created_at = Some(current_timestamp());
    }
    user_meta.description = Some(description.to_string());
    update_user_metadata(repo, selector, user_meta)
}

pub fn add_label(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    label: &str,
) -> Result<StaircaseRecord> {
    let mut user_meta = get_user_metadata(repo, selector)?;
    if !user_meta.labels.contains(&label.to_string()) {
        user_meta.labels.push(label.to_string());
    }
    update_user_metadata(repo, selector, user_meta)
}

pub fn remove_label(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    label: &str,
) -> Result<StaircaseRecord> {
    let mut user_meta = get_user_metadata(repo, selector)?;
    user_meta.labels.retain(|l| l != label);
    update_user_metadata(repo, selector, user_meta)
}

pub fn add_link(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    link: StaircaseLink,
) -> Result<StaircaseRecord> {
    let mut user_meta = get_user_metadata(repo, selector)?;
    user_meta.links.retain(|l| l.id != link.id);
    user_meta.links.push(link);
    update_user_metadata(repo, selector, user_meta)
}

pub fn get_step_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    step_key: &str,
) -> Result<StepMetadata> {
    let user_meta = get_user_metadata(repo, selector)?;
    let meta = selector.staircase.metadata();

    let key = resolve_step_key(meta, step_key)?;
    Ok(user_meta
        .step_metadata
        .get(&key)
        .cloned()
        .unwrap_or_default())
}

pub fn update_step_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    step_key: &str,
    mut step_meta: StepMetadata,
) -> Result<StaircaseRecord> {
    let mut user_meta = get_user_metadata(repo, selector)?;
    let meta = selector.staircase.metadata();

    let key = resolve_step_key(meta, step_key)?;

    step_meta.labels.sort();
    step_meta.labels.dedup();

    user_meta.step_metadata.insert(key, step_meta);
    update_user_metadata(repo, selector, user_meta)
}

fn resolve_step_key(meta: &StaircaseMetadata, step_key: &str) -> Result<String> {
    if let Ok(ordinal) = step_key.parse::<usize>() {
        if ordinal >= 1 && ordinal <= meta.steps.len() {
            let step = &meta.steps[ordinal - 1];
            return Ok(if !step.id.is_empty() {
                step.id.clone()
            } else {
                step.name.clone()
            });
        }
    }

    for step in &meta.steps {
        if step.id == step_key || step.name == step_key {
            return Ok(if !step.id.is_empty() {
                step.id.clone()
            } else {
                step.name.clone()
            });
        }
    }

    Ok(step_key.to_string())
}

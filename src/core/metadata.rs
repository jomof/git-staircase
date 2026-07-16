use crate::core::persistence;
use crate::core::refs::StaircaseRefs;
use crate::core::resolved::ResolvedSelector;
use crate::core::utils::current_timestamp;
use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{
    LifecycleState, StaircaseLifecycle, StaircaseLink, StaircaseRecord, StaircaseUserMetadata,
    StepMetadata,
};

pub fn get_user_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
) -> Result<StaircaseUserMetadata> {
    if selector.staircase.is_implicit() {
        return Ok(StaircaseUserMetadata::default());
    }
    let meta = selector.staircase.metadata();
    let record_ref = StaircaseRefs::record(
        &meta.id,
        meta.lifecycle
            .as_ref()
            .map(|l| l.state)
            .unwrap_or(LifecycleState::Active),
    );
    let record = persistence::read_record(repo, &record_ref)?;

    Ok(record.user_metadata)
}

pub fn get_user_metadata_snapshot(
    repo: &GitRepo,
    selector: &ResolvedSelector,
) -> Result<(StaircaseUserMetadata, String)> {
    if selector.staircase.is_implicit() {
        return Ok((StaircaseUserMetadata::default(), String::new()));
    }
    let record = read_selected_record(repo, selector)?;
    Ok((record.user_metadata, record.record_oid))
}

pub fn update_user_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    new_user_meta: StaircaseUserMetadata,
) -> Result<StaircaseRecord> {
    let (_current_meta, expected_oid) = get_user_metadata_snapshot(repo, selector)?;
    update_user_metadata_expected(repo, selector, new_user_meta, &expected_oid)
}

pub fn update_user_metadata_expected(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    mut new_user_meta: StaircaseUserMetadata,
    expected_record_oid: &str,
) -> Result<StaircaseRecord> {
    let meta = selector.staircase.metadata();
    let mut record = if expected_record_oid.is_empty() && selector.staircase.is_implicit() {
        StaircaseRecord {
            record_oid: String::new(),
            structure_oid: String::new(),
            metadata_oid: String::new(),
            lifecycle_oid: String::new(),
            archive_manifest_oid: None,
            metadata: meta.clone(),
            user_metadata: StaircaseUserMetadata::default(),
            lifecycle: StaircaseLifecycle::default(),
            archive_manifest: None,
        }
    } else {
        persistence::read_record(repo, expected_record_oid)?
    };

    if record.metadata.id.starts_with("implicit@") {
        record.metadata = crate::core::adopt(repo, &record.metadata)?;
    }
    if record.metadata.id != meta.id {
        return Err(crate::StaircaseError::ConcurrentRecordUpdate {
            reference: StaircaseRefs::state_record(&meta.id),
            expected: expected_record_oid.into(),
            actual: record.record_oid,
        });
    }
    record.metadata.name = meta.name.clone();

    new_user_meta.labels.sort();
    new_user_meta.labels.dedup();
    new_user_meta.updated_at = Some(current_timestamp());

    let updated_record = persistence::write_record(
        repo,
        &record.metadata,
        &new_user_meta,
        &record.lifecycle,
        record.archive_manifest.as_ref(),
        Some(expected_record_oid),
        true,
    )?;

    Ok(updated_record)
}

fn read_selected_record(repo: &GitRepo, selector: &ResolvedSelector) -> Result<StaircaseRecord> {
    let meta = selector.staircase.metadata();
    let record_ref = StaircaseRefs::record(
        &meta.id,
        meta.lifecycle
            .as_ref()
            .map(|l| l.state)
            .unwrap_or(LifecycleState::Active),
    );
    persistence::read_record(repo, &record_ref)
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

    let key = resolve_step_key(selector, step_key)?;
    Ok(user_meta
        .step_metadata
        .get(&key)
        .cloned()
        .unwrap_or_default())
}

pub fn get_step_metadata_snapshot(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    step_key: &str,
) -> Result<(StepMetadata, String)> {
    let (user_meta, record_oid) = get_user_metadata_snapshot(repo, selector)?;
    let key = resolve_step_key(selector, step_key)?;
    Ok((
        user_meta
            .step_metadata
            .get(&key)
            .cloned()
            .unwrap_or_default(),
        record_oid,
    ))
}

pub fn update_step_metadata_expected(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    step_key: &str,
    mut step_meta: StepMetadata,
    expected_record_oid: &str,
) -> Result<StaircaseRecord> {
    let record = persistence::read_record(repo, expected_record_oid)?;
    let mut user_meta = record.user_metadata;
    let key = resolve_step_key(selector, step_key)?;
    step_meta.labels.sort();
    step_meta.labels.dedup();
    user_meta.step_metadata.insert(key, step_meta);
    update_user_metadata_expected(repo, selector, user_meta, expected_record_oid)
}

pub fn update_step_metadata(
    repo: &GitRepo,
    selector: &ResolvedSelector,
    step_key: &str,
    mut step_meta: StepMetadata,
) -> Result<StaircaseRecord> {
    let mut user_meta = get_user_metadata(repo, selector)?;

    let key = resolve_step_key(selector, step_key)?;

    step_meta.labels.sort();
    step_meta.labels.dedup();

    user_meta.step_metadata.insert(key, step_meta);
    update_user_metadata(repo, selector, user_meta)
}

fn resolve_step_key(sel: &ResolvedSelector, step_arg: &str) -> Result<String> {
    let idx = super::resolution::resolve_step(sel, Some(step_arg))?;
    let step = &sel.metadata().steps[idx];
    Ok(if !step.id.is_empty() {
        step.id.clone()
    } else {
        step.name.clone()
    })
}

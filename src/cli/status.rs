use super::StaircaseSelectorArgs;
use crate::GitRepo;
use crate::core;
use crate::model::StaircaseStatus;

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> anyhow::Result<StaircaseStatus> {
    let rs = super::resolve_rs(repo, &staircase)?;
    Ok(core::get_status_metadata(
        repo,
        rs.metadata().clone(),
        !rs.is_managed(),
    )?)
}

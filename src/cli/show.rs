use super::StaircaseSelectorArgs;
use crate::GitRepo;
use crate::model::StaircaseMetadata;

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> anyhow::Result<StaircaseMetadata> {
    let rs = super::resolve_rs(repo, &staircase)?;
    Ok(rs.metadata().clone())
}

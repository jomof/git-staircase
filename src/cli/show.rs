use super::StaircaseSelectorArgs;
use crate::GitRepo;
use crate::model::StaircaseMetadata;

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> anyhow::Result<StaircaseMetadata> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    Ok(rs.metadata().clone())
}

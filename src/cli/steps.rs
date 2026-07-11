use super::{StaircaseSelectorArgs, StepsList};
use crate::GitRepo;

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> anyhow::Result<StepsList> {
    let rs = staircase.resolve(repo)?;
    Ok(StepsList(rs.metadata().steps.clone()))
}

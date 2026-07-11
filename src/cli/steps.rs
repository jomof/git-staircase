use super::{StaircaseSelectorArgs, StepsList};
use crate::GitRepo;

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> anyhow::Result<StepsList> {
    let rs = super::resolve_rs(repo, &staircase)?;
    Ok(StepsList(rs.metadata().steps.clone()))
}

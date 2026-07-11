use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> anyhow::Result<Success> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    core::restack(repo, &rs)?;
    Ok(Success::new("Restacked staircase."))
}

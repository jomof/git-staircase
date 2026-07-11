use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> anyhow::Result<Success> {
    let rs = super::resolve_rs(repo, &staircase)?;
    core::restack(repo, &rs)?;
    Ok(Success::new("Restacked staircase."))
}

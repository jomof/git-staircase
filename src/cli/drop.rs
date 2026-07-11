use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::anyhow;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
) -> anyhow::Result<Success> {
    let rs = super::resolve_rs(repo, &staircase)?;
    let step_idx = step.ok_or_else(|| anyhow!("--step (index) must be provided"))?;
    if step_idx == 0 {
        return Err(anyhow!("Step index must be 1-based (got 0)"));
    }
    core::drop(repo, &rs, step_idx - 1)?;
    Ok(Success::new("Dropped step."))
}

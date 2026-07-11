use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::anyhow;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    at: String,
    step_name: Option<String>,
) -> anyhow::Result<Success> {
    let rs = staircase.resolve(repo)?;
    let step_num = if let Some(s) = step {
        s
    } else {
        rs.step_index.map(|i| i + 1).ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?
    };

    if step_num == 0 {
        return Err(anyhow!("Step number must be 1-based"));
    }
    core::split(repo, &rs.staircase, step_num - 1, &at, step_name.as_deref())?;
    Ok(Success::new(format!(
        "Split step {} of staircase '{}' at {}.",
        step_num,
        rs.metadata().name,
        at
    )))
}

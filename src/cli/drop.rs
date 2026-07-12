use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Drop {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// Step number (1-based). Can be part of the staircase name (e.g. name:1)
    #[arg(long)]
    pub step: Option<usize>,
}

impl super::Command for Drop {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.staircase.clone(), self.step)?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
) -> Result<Success> {
    let rs = staircase.resolve(repo)?;
    let step_num = if let Some(s) = step {
        s
    } else {
        rs.step_index.map(|i| i + 1).ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?
    };

    if step_num == 0 {
        return Err(anyhow!("Step number must be 1-based"));
    }
    core::drop(repo, &rs.staircase, step_num - 1)?;
    Ok(Success::new(format!(
        "Dropped step {} of staircase '{}'.",
        step_num,
        rs.metadata().name
    )))
}

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
) -> Result<Success> {
    run_internal(repo, staircase, step)
}

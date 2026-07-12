use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Split {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// Step number (1-based). Can be part of the staircase name (e.g. name:1)
    #[arg(long)]
    pub step: Option<usize>,
    #[arg(long)]
    pub at: String,
    /// Name of the new step.
    #[arg(long)]
    pub step_name: Option<String>,
    #[arg(long)]
    pub no_ref: bool,
}

impl super::Command for Split {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(
            repo,
            self.staircase.clone(),
            self.step,
            self.at.clone(),
            self.step_name.clone(),
            self.no_ref,
        )?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    at: String,
    step_name: Option<String>,
    no_ref: bool,
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
    core::split(
        repo,
        &rs.staircase,
        step_num - 1,
        &at,
        step_name.as_deref(),
        core::SplitOptions { no_ref },
    )?;
    Ok(Success::new(format!(
        "Split step {} of staircase '{}' at {}.",
        step_num,
        rs.metadata().name,
        at
    )))
}

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    at: String,
    step_name: Option<String>,
    no_ref: bool,
) -> Result<Success> {
    run_internal(repo, staircase, step, at, step_name, no_ref)
}

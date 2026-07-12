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
        let rs = self.staircase.resolve(repo)?;
        let step_num = if let Some(s) = self.step {
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
            &self.at,
            self.step_name.as_deref(),
            core::SplitOptions {
                no_ref: self.no_ref,
            },
        )?;
        Ok(Box::new(Success::new(format!(
            "Split step {} of staircase '{}' at {}.",
            step_num,
            rs.metadata().name,
            self.at
        ))))
    }
}

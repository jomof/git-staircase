use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use crate::core::DropOptions;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Drop {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub restack: bool,
}

impl super::Command for Drop {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let step_index = self
            .staircase
            .name
            .as_ref()
            .and_then(|n| rs.metadata().steps.iter().position(|s| s.name == *n))
            .ok_or_else(|| anyhow!("Could not determine which step to drop. Please provide a step name or use an explicit selector."))?;

        core::drop(
            repo,
            &rs,
            step_index,
            DropOptions {
                restack: self.restack,
                leave_descendants_stale: !self.restack,
            },
        )?;

        Ok(Box::new(Success::new(format!(
            "Dropped step {} from staircase '{}'",
            step_index + 1,
            rs.metadata().name
        ))))
    }
}

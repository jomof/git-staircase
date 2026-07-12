use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Restack {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub from: Option<String>,
}

impl super::Command for Restack {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        core::restack(
            repo,
            &rs,
            core::RebaseOptions {
                leave_upper_steps_stale: false,
            },
        )?;
        Ok(Box::new(Success::new(format!(
            "Restacked staircase '{}'",
            rs.metadata().name
        ))))
    }
}

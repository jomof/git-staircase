use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Rebase {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long = "to")]
    pub to: String,
    #[arg(long = "leave-upper-steps-stale")]
    pub leave_upper_steps_stale: bool,
}

impl super::Command for Rebase {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        core::rebase(
            repo,
            &rs,
            &self.to,
            core::RebaseOptions {
                leave_upper_steps_stale: self.leave_upper_steps_stale,
            },
        )?;
        Ok(Box::new(Success::new(format!(
            "Rebased staircase onto {}",
            self.to
        ))))
    }
}

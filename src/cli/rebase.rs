use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Rebase {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long = "leave-upper-steps-stale")]
    pub leave_upper_steps_stale: bool,
    #[arg(long)]
    pub dry_run: bool,
}

impl super::Command for Rebase {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let target = self
            .staircase
            .base
            .onto
            .as_deref()
            .unwrap_or(&rs.metadata().symbolic_integration_target);
        repo.resolve_commit(target)?;
        core::rebase_with_dry_run(
            repo,
            &rs,
            target,
            core::RebaseOptions {
                leave_upper_steps_stale: self.leave_upper_steps_stale,
            },
            self.dry_run,
        )?;
        Ok(Box::new(Success::new(format!(
            "Rebased staircase onto {}",
            target
        ))))
    }
}

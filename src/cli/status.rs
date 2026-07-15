use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Status {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// Include status from all worktrees.
    #[arg(long)]
    pub all_worktrees: bool,
}

impl super::Command for Status {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve_with_default(repo, "HEAD")?;
        let status = core::get_status_metadata(
            repo,
            rs.metadata().clone(),
            !rs.is_managed(),
            self.all_worktrees,
        )?;
        // Note: all_worktrees flag should be passed to get_status_metadata if core supported it.
        // For now, we are just adding it to the CLI.
        Ok(Box::new(status))
    }
}

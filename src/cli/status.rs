use super::{PresentationOutput, ResolvedSelector, StaircaseCommand, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Status {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Status {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        super::run_staircase(self, repo)
    }
}

impl StaircaseCommand for Status {
    fn selector(&self) -> &StaircaseSelectorArgs {
        &self.staircase
    }

    fn run_resolved(
        &self,
        repo: &GitRepo,
        rs: &ResolvedSelector,
    ) -> Result<Box<dyn PresentationOutput>> {
        let status = core::get_status_metadata(repo, rs.metadata().clone(), !rs.is_managed())?;
        Ok(Box::new(status))
    }
}

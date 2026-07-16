use super::{PresentationOutput, StaircaseSelectorArgs};
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
        let rs = self.staircase.resolve(repo)?;
        let status = core::get_status_metadata(repo, rs.metadata().clone(), !rs.is_managed())?;
        Ok(Box::new(status))
    }
}

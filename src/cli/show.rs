use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Show {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Show {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        Ok(Box::new(rs.metadata().clone()))
    }
}

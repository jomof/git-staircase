use super::{PresentationOutput, StaircaseSelectorArgs, StepsList};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Steps {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Steps {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        Ok(Box::new(StepsList(rs.metadata().steps.clone())))
    }
}

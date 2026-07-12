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
        let result = run_internal(repo, self.staircase.clone())?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<StepsList> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    Ok(StepsList(rs.metadata().steps.clone()))
}

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<StepsList> {
    run_internal(repo, staircase)
}

use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Restack {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Restack {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.staircase.clone())?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<Success> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    core::restack(repo, &rs)?;
    Ok(Success::new("Restacked staircase."))
}

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<Success> {
    run_internal(repo, staircase)
}

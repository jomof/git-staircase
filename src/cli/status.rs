use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use crate::model::StaircaseStatus;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Status {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Status {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.staircase.clone())?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<StaircaseStatus> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    Ok(core::get_status_metadata(
        repo,
        rs.metadata().clone(),
        !rs.is_managed(),
    )?)
}

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<StaircaseStatus> {
    run_internal(repo, staircase)
}

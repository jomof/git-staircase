use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::model::StaircaseMetadata;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Show {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Show {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.staircase.clone())?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<StaircaseMetadata> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    Ok(rs.metadata().clone())
}

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs) -> Result<StaircaseMetadata> {
    run_internal(repo, staircase)
}

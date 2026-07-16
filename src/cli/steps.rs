use super::{
    PresentationOutput, ResolvedSelector, StaircaseCommand, StaircaseSelectorArgs, StepsList,
};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Steps {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Steps {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        super::run_staircase(self, repo)
    }
}

impl StaircaseCommand for Steps {
    fn selector(&self) -> &StaircaseSelectorArgs {
        &self.staircase
    }

    fn run_resolved(
        &self,
        _repo: &GitRepo,
        rs: &ResolvedSelector,
    ) -> Result<Box<dyn PresentationOutput>> {
        Ok(Box::new(StepsList(rs.metadata().steps.clone())))
    }
}

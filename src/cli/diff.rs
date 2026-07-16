use super::{
    PlainOutput, PresentationOutput, ResolvedSelector, StaircaseCommand, StaircaseSelectorArgs,
};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Diff {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Diff {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        super::run_staircase(self, repo)
    }
}

impl StaircaseCommand for Diff {
    fn selector(&self) -> &StaircaseSelectorArgs {
        &self.staircase
    }

    fn run_resolved(
        &self,
        repo: &GitRepo,
        rs: &ResolvedSelector,
    ) -> Result<Box<dyn PresentationOutput>> {
        let m = rs.metadata();
        let target = &m.target;
        let tip = &m.steps.last().expect("Staircase has no steps").cut;

        let output = repo.run(&["diff", &format!("{}..{}", target, tip)])?;
        Ok(Box::new(PlainOutput(output)))
    }
}

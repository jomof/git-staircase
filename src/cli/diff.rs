use super::{PlainOutput, PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Diff {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Diff {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let m = rs.metadata();
        let symbolic_integration_target = &m.symbolic_integration_target;
        let tip = &m.steps.last().expect("Staircase has no steps").cut;

        let output = repo.run(&["diff", &format!("{}..{}", symbolic_integration_target, tip)])?;
        Ok(Box::new(PlainOutput(output)))
    }
}

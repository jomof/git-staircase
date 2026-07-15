use super::{PlainOutput, PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Graph {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Graph {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let m = rs.metadata();
        let target = &m.symbolic_integration_target;
        let tip = &m.steps.last().expect("Staircase has no steps").cut;

        let output = repo.run(&[
            "log",
            "--graph",
            "--oneline",
            &format!("{}..{}", target, tip),
        ])?;
        Ok(Box::new(PlainOutput(output)))
    }
}

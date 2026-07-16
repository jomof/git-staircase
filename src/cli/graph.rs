use super::{PresentationOutput, ResolvedSelector, StaircaseCommand, StaircaseSelectorArgs};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Graph {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Graph {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        super::run_staircase(self, repo)
    }
}

impl StaircaseCommand for Graph {
    fn selector(&self) -> &StaircaseSelectorArgs {
        &self.staircase
    }

    fn run_resolved(
        &self,
        repo: &GitRepo,
        rs: &ResolvedSelector,
    ) -> Result<Box<dyn PresentationOutput>> {
        let output = repo.run(&[
            "log",
            "--graph",
            "--oneline",
            &format!(
                "{}..{}",
                rs.metadata().target,
                rs.metadata().steps.last().unwrap().cut
            ),
        ])?;
        Ok(Box::new(GraphOutput(output)))
    }
}

#[derive(serde::Serialize)]
struct GraphOutput(String);

impl super::ToPresentation for GraphOutput {
    fn to_presentation(&self) -> super::Presentation {
        super::Presentation::Plain(self.0.clone())
    }
}

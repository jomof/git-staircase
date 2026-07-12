use super::formatting::{ToHuman, ToPorcelain};
use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use crate::model::VerificationResult;
use anyhow::Result;
use serde::Serialize;

#[derive(clap::Args, Clone, Debug)]
pub struct Verify {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub aggregate: bool,
    #[arg(long)]
    pub each_prefix: bool,
    #[arg(long)]
    pub build_command: Option<String>,
    #[arg(long)]
    pub test_command: Option<String>,
}

impl super::Command for Verify {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let aggregate_opt = if self.aggregate { Some(true) } else { None };
        let each_prefix_opt = if self.each_prefix { Some(true) } else { None };

        let rs = self.staircase.resolve(repo)?;
        let results = core::verify(
            repo,
            &rs,
            self.build_command.clone(),
            self.test_command.clone(),
            aggregate_opt,
            each_prefix_opt,
        )?;
        Ok(Box::new(VerificationResults(results)))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct VerificationResults(pub Vec<VerificationResult>);

impl ToHuman for VerificationResults {
    fn to_human(&self) -> String {
        self.0.to_human()
    }
}

impl ToPorcelain for VerificationResults {
    fn to_porcelain(&self) -> String {
        self.0.to_porcelain()
    }
}

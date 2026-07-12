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
        let result = run_internal(
            repo,
            self.staircase.clone(),
            self.aggregate,
            self.each_prefix,
            self.build_command.clone(),
            self.test_command.clone(),
        )?;
        Ok(Box::new(VerificationResults(result)))
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

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    aggregate: bool,
    each_prefix: bool,
    build_command: Option<String>,
    test_command: Option<String>,
) -> Result<Vec<VerificationResult>> {
    let aggregate_opt = if aggregate { Some(true) } else { None };
    let each_prefix_opt = if each_prefix { Some(true) } else { None };

    let rs = staircase.resolve(repo)?;
    let rs = &rs;

    Ok(core::verify(
        repo,
        &rs,
        build_command,
        test_command,
        aggregate_opt,
        each_prefix_opt,
    )?)
}

pub fn run(
    repo: &GitRepo,
    _format: super::OutputFormat,
    staircase: StaircaseSelectorArgs,
    aggregate: bool,
    each_prefix: bool,
    build_command: Option<String>,
    test_command: Option<String>,
) -> Result<()> {
    let results = run_internal(
        repo,
        staircase,
        aggregate,
        each_prefix,
        build_command,
        test_command,
    )?;
    println!("{}", results.to_human());
    Ok(())
}

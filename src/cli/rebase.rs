use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Rebase {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long = "to")]
    pub to: String,
}

impl super::Command for Rebase {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.staircase.clone(), self.to.clone())?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    to: String,
) -> Result<Success> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    core::rebase(repo, &rs, &to)?;
    Ok(Success::new(format!("Rebased staircase onto {}", to)))
}

pub fn run(repo: &GitRepo, staircase: StaircaseSelectorArgs, to: String) -> Result<Success> {
    run_internal(repo, staircase, to)
}

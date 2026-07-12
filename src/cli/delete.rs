use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Delete {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub delete_branches: bool,
}

impl super::Command for Delete {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.staircase.clone(), self.delete_branches)?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    delete_branches: bool,
) -> Result<Success> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    core::delete(repo, &rs.metadata().id, delete_branches)?;
    Ok(Success::new(format!(
        "Deleted staircase '{}'.",
        rs.metadata().name
    )))
}

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    delete_branches: bool,
) -> Result<Success> {
    run_internal(repo, staircase, delete_branches)
}

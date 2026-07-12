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
    #[arg(long = "leave-upper-steps-stale")]
    pub leave_upper_steps_stale: bool,
}

impl super::Command for Rebase {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(
            repo,
            self.staircase.clone(),
            self.to.clone(),
            self.leave_upper_steps_stale,
        )?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    to: String,
    leave_upper_steps_stale: bool,
) -> Result<Success> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    core::rebase(
        repo,
        &rs,
        &to,
        core::RebaseOptions {
            leave_upper_steps_stale,
        },
    )?;
    Ok(Success::new(format!("Rebased staircase onto {}", to)))
}

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    to: String,
    leave_upper_steps_stale: bool,
) -> Result<Success> {
    run_internal(repo, staircase, to, leave_upper_steps_stale)
}

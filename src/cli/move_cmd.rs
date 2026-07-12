use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Move {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub from: usize,
    #[arg(long)]
    pub to: usize,
    pub commits: Vec<String>,
}

impl super::Command for Move {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(
            repo,
            self.staircase.clone(),
            self.from,
            self.to,
            self.commits.clone(),
        )?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    from: usize,
    to: usize,
    commits: Vec<String>,
) -> Result<Success> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    core::move_commits(repo, &rs, from - 1, to - 1, &commits)?;
    Ok(Success::new("Moved commits."))
}

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    from: usize,
    to: usize,
    commits: Vec<String>,
) -> Result<Success> {
    run_internal(repo, staircase, from, to, commits)
}

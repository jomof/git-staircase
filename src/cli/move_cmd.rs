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
        let rs = self.staircase.resolve(repo)?;
        core::move_commits(repo, &rs, self.from - 1, self.to - 1, &self.commits)?;
        Ok(Box::new(Success::new("Moved commits.")))
    }
}

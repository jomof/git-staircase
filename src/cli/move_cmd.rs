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
    #[arg(long)]
    pub dry_run: bool,
}

impl super::Command for Move {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        if self.from == 0 || self.to == 0 {
            return Err(anyhow::anyhow!("step numbers are 1-based"));
        }
        if self.from > rs.metadata().steps.len() || self.to > rs.metadata().steps.len() {
            return Err(anyhow::anyhow!("step is out of range"));
        }
        for commit in &self.commits {
            repo.resolve_commit(commit)?;
        }
        core::move_commits_with_dry_run(
            repo,
            &rs,
            self.from - 1,
            self.to - 1,
            &self.commits,
            self.dry_run,
        )?;
        Ok(Box::new(Success::new("Moved commits.")))
    }
}

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
        let rs = self.staircase.resolve(repo)?;
        core::delete(repo, &rs.metadata().id, self.delete_branches)?;
        Ok(Box::new(Success::new(format!(
            "Deleted staircase '{}'.",
            rs.metadata().name
        ))))
    }
}

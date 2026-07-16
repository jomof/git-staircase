use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Delete {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long, alias = "delete-refs")]
    pub delete_branches: bool,
    #[arg(long)]
    pub delete_step_refs: bool,
    #[arg(long)]
    pub keep_step_refs: bool,
}

impl super::Command for Delete {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let delete_refs = (self.delete_branches || self.delete_step_refs) && !self.keep_step_refs;
        core::delete(repo, &rs.metadata().id, delete_refs)?;
        Ok(Box::new(Success::new(format!(
            "Deleted staircase '{}'.",
            rs.metadata().name
        ))))
    }
}

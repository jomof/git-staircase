use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Delete {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub delete_owned_branches: bool,
    #[arg(long)]
    pub delete_materializing_refs: bool,
    #[arg(long)]
    pub keep_owned_branches: bool,
}

impl super::Command for Delete {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;

        if !rs.is_managed() && !self.delete_materializing_refs {
            return Err(anyhow!(
                "'{}' is an implicit staircase; use --delete-materializing-refs to delete its branches.",
                rs.metadata().name
            ));
        }

        let delete_refs = if rs.is_managed() {
            self.delete_owned_branches && !self.keep_owned_branches
        } else {
            self.delete_materializing_refs
        };

        core::delete(repo, &rs, delete_refs)?;
        Ok(Box::new(Success::new(format!(
            "Deleted staircase '{}'.",
            rs.metadata().name
        ))))
    }
}

use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Status {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Status {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = match self.staircase.resolve_opt(repo)? {
            Some(rs) => rs,
            None => core::resolve_staircase(repo, "HEAD", self.staircase.base.onto.as_deref())?
                .ok_or_else(|| anyhow!("Could not infer current staircase from HEAD. Please provide a selector."))?,
        };
        let status = core::get_status_metadata(repo, rs.metadata().clone(), !rs.is_managed())?;
        Ok(Box::new(status))
    }
}

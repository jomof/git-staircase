use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use crate::model::LandingPolicy;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Land {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// Override landing policy.
    #[arg(long)]
    pub policy: Option<LandingPolicy>,
}

impl super::Command for Land {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        core::land(
            repo,
            &rs,
            core::LandOptions {
                policy: self.policy,
            },
        )?;
        Ok(Box::new(Success::new(format!(
            "Landed staircase '{}'",
            rs.metadata().name
        ))))
    }
}

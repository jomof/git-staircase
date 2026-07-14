use crate::GitRepo;
use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs, StructuredOutput};
use crate::core;
use anyhow::Result;
use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct Normalize {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub dry_run: bool,
}

impl Command for Normalize {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let selector = self.selector.resolve(repo)?;
        Ok(Box::new(StructuredOutput(core::normalize(
            repo,
            &selector,
            self.dry_run,
        )?)))
    }
}

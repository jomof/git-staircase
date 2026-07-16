use crate::GitRepo;
use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs};
use crate::core;
use anyhow::Result;
use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct Append {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub commits: String,
    #[arg(long)]
    pub new_step: bool,
    #[arg(long, requires = "new_step")]
    pub branch: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
}

impl Command for Append {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let selector = self.selector.resolve(repo)?;
        let result = core::append(
            repo,
            &selector,
            &self.commits,
            self.new_step,
            self.branch.as_deref(),
            self.dry_run,
        )?;
        Ok(Box::new(result))
    }
}

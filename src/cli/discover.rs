use super::PresentationOutput;
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Discover {
    #[arg(long)]
    pub onto: Option<String>,
    #[arg(long)]
    pub refs: Option<String>,
    #[arg(long, short)]
    pub families: bool,
    #[arg(long)]
    pub top: Option<String>,
}

impl super::Command for Discover {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let results = core::discover(
            repo,
            self.onto.as_deref(),
            self.refs.as_deref(),
            self.families,
        )?;
        Ok(Box::new(results))
    }

    fn requires_clear_operation(&self) -> bool {
        false
    }
}

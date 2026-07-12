use super::PresentationOutput;
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Discover {
    #[arg(long)]
    pub onto: Option<String>,
    #[arg(long, short)]
    pub families: bool,
}

impl super::Command for Discover {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let results = core::discover(repo, self.onto.as_deref(), None, self.families)?;
        Ok(Box::new(results))
    }
}

use crate::GitRepo;
use crate::cli::{Command, PresentationOutput, RequiredStaircaseSelector, StaircaseSelectorArgs};
use crate::core;
use anyhow::Result;
use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct Name {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    pub new_name: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct Rename {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    pub new_name: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct Unname {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub dry_run: bool,
}

impl Command for Name {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let selector = self.selector.resolve(repo)?;
        Ok(Box::new(core::name_staircase(
            repo,
            &selector,
            &self.new_name,
            self.dry_run,
        )?))
    }
}

impl Command for Rename {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let selector = self.selector.resolve(repo)?;
        Ok(Box::new(core::rename_staircase(
            repo,
            &selector,
            &self.new_name,
            self.dry_run,
        )?))
    }
}

impl Command for Unname {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let selector = self.selector.resolve(repo)?;
        Ok(Box::new(core::unname_staircase(
            repo,
            &selector,
            self.dry_run,
        )?))
    }
}

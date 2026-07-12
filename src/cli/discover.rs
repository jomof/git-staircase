use super::{PresentationOutput, ToHuman, ToPorcelain};
use crate::GitRepo;
use crate::core;
use crate::model::Discovery;
use anyhow::Result;
use serde::Serialize;

#[derive(clap::Args, Clone, Debug)]
pub struct Discover {
    #[arg(long)]
    pub onto: Option<String>,
}

impl super::Command for Discover {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.onto.clone())?;
        Ok(Box::new(DiscoveryList(result)))
    }
}

pub fn run_internal(repo: &GitRepo, onto: Option<String>) -> Result<Vec<Discovery>> {
    Ok(core::discover(repo, onto.as_deref())?)
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct DiscoveryList(pub Vec<Discovery>);

impl ToHuman for DiscoveryList {
    fn to_human(&self) -> String {
        if self.0.is_empty() {
            "No potential staircases discovered.".to_string()
        } else {
            self.0.to_human()
        }
    }
}

impl ToPorcelain for DiscoveryList {
    fn to_porcelain(&self) -> String {
        self.0.to_porcelain()
    }
}

pub fn run(repo: &GitRepo, _format: super::OutputFormat, onto: Option<String>) -> Result<()> {
    let result = run_internal(repo, onto)?;
    println!("{}", result.to_human()); // This is just for backward compatibility if needed, but we should use dispatch
    Ok(())
}

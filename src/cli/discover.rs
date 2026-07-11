use super::{OutputFormat, print_output};
use crate::GitRepo;
use git_staircase::core;

pub fn run(repo: &GitRepo, format: OutputFormat, onto: Option<String>) -> anyhow::Result<()> {
    let discovered = core::discover(repo, onto.as_deref())?;
    if discovered.is_empty() && matches!(format, OutputFormat::Human) {
        println!("No potential staircases discovered.");
        return Ok(());
    }
    print_output(format, &discovered)
}

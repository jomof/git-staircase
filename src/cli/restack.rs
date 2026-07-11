use crate::GitRepo;
use super::OutputFormat;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
    core::restack(repo, &rs)?;
    if matches!(format, OutputFormat::Human) {
        println!("Restacked staircase.");
    }
    Ok(())
}

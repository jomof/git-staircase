use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    core::restack(repo, &rs)?;
    if matches!(format, OutputFormat::Human) {
        println!("Restacked staircase.");
    }
    Ok(())
}

use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    to: String,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    core::rebase(repo, &rs, &to)?;
    if matches!(format, OutputFormat::Human) {
        println!("Rebased staircase onto {}", to);
    }
    Ok(())
}

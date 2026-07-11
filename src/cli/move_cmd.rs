use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    from: usize,
    to: usize,
    commits: Vec<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    core::move_commits(repo, &rs, from - 1, to - 1, &commits)?;
    if matches!(format, OutputFormat::Human) {
        println!("Moved commits.");
    }
    Ok(())
}

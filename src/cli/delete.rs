use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    _format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    delete_branches: bool,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    core::delete(repo, &rs.metadata().id, delete_branches)?;
    println!("Deleted staircase '{}'.", rs.metadata().name);
    Ok(())
}

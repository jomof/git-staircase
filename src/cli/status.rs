use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    let status = core::get_status_metadata(repo, rs.metadata().clone(), !rs.is_managed())?;
    super::print_output(format, &status)
}

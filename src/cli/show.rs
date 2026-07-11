use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    super::print_output(format, rs.metadata())
}

use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    delete_branches: bool,
) -> anyhow::Result<Success> {
    let rs = super::resolve_rs(repo, &staircase)?;
    core::delete(repo, &rs.metadata().id, delete_branches)?;
    Ok(Success::new(format!(
        "Deleted staircase '{}'.",
        rs.metadata().name
    )))
}

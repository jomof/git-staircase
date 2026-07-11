use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    from: usize,
    to: usize,
    commits: Vec<String>,
) -> anyhow::Result<Success> {
    let rs = staircase.resolve(repo)?;
    core::move_commits(repo, &rs, from - 1, to - 1, &commits)?;
    Ok(Success::new("Moved commits."))
}

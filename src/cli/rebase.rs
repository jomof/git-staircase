use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    to: String,
) -> anyhow::Result<Success> {
    let rs = staircase.resolve(repo)?;
    core::rebase(repo, &rs, &to)?;
    Ok(Success::new(format!("Rebased staircase onto {}", to)))
}

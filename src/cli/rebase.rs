use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    to: String,
) -> anyhow::Result<Success> {
    let rs = super::resolve_rs(repo, &staircase)?;
    core::rebase(repo, &rs, &to)?;
    Ok(Success::new(format!("Rebased staircase onto {}", to)))
}

use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;

pub fn run(
    repo: &GitRepo,
    _format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    git_args: Vec<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    let mut args = vec!["log", "--graph", "--oneline"];
    let range = format!(
        "{}..{}",
        rs.metadata().target,
        rs.metadata().steps.last().unwrap().cut
    );
    args.push(&range);
    for arg in &git_args {
        args.push(arg);
    }
    repo.run_interactive(&args)?;
    Ok(())
}

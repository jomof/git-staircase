use super::{OutputFormat, PlainOutput, StaircaseSelectorArgs};
use crate::GitRepo;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    git_args: Vec<String>,
) -> anyhow::Result<()> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
    let range = format!(
        "{}..{}",
        rs.metadata().target,
        rs.metadata().steps.last().unwrap().cut
    );

    match format {
        OutputFormat::Human => {
            let mut args = vec!["diff"];
            args.push(&range);
            for arg in &git_args {
                args.push(arg);
            }
            repo.run_interactive(&args)?;
            Ok(())
        }
        _ => {
            let mut args = vec!["diff"];
            args.push(&range);
            for arg in &git_args {
                args.push(arg);
            }
            let output = repo.run(&args)?;
            super::dispatch(format, Ok(PlainOutput(output)))
        }
    }
}

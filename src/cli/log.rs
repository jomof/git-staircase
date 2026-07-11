use super::{CommitInfo, LogOutput, OutputFormat, StaircaseSelectorArgs};
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
            let mut args = vec!["log"];
            args.push(&range);
            for arg in &git_args {
                args.push(arg);
            }
            repo.run_interactive(&args)?;
            Ok(())
        }
        _ => {
            let mut args = vec!["log", "--oneline"];
            args.push(&range);
            // We might want to allow some git_args here, but for structured output
            // we'll stick to oneline for now to keep it simple and parseable.
            let output = repo.run(&args)?;
            let mut commits = Vec::new();
            for line in output.lines() {
                if let Some((hash, subject)) = line.split_once(' ') {
                    commits.push(CommitInfo {
                        hash: hash.to_string(),
                        subject: subject.to_string(),
                    });
                }
            }
            super::dispatch(format, Ok(LogOutput(commits)))
        }
    }
}

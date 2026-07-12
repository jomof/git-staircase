use super::{CommitInfo, LogOutput, OutputFormat, PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Log {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(last = true)]
    pub git_args: Vec<String>,
}

impl super::Command for Log {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let rs = &rs;
        let range = format!(
            "{}..{}",
            rs.metadata().target,
            rs.metadata().steps.last().unwrap().cut
        );

        let mut args = vec!["log", "--oneline"];
        args.push(&range);
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

        Ok(Box::new(LogResult {
            range,
            git_args: self.git_args.clone(),
            commits: LogOutput(commits),
        }))
    }
}

pub struct LogResult {
    pub range: String,
    pub git_args: Vec<String>,
    pub commits: LogOutput,
}

impl PresentationOutput for LogResult {
    fn present(&self, format: OutputFormat, repo: &GitRepo) -> Result<()> {
        match format {
            OutputFormat::Human => {
                let mut args = vec!["log"];
                args.push(&self.range);
                for arg in &self.git_args {
                    args.push(arg);
                }
                repo.run_interactive(&args)?;
                Ok(())
            }
            _ => self.commits.present(format, repo),
        }
    }
    fn to_json(&self) -> Result<String> {
        self.commits.to_json()
    }
}

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    git_args: Vec<String>,
) -> Result<()> {
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
            super::dispatch(format, repo, Ok(Box::new(LogOutput(commits))))
        }
    }
}

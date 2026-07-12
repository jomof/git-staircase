use super::{OutputFormat, PlainOutput, PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Graph {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(last = true)]
    pub git_args: Vec<String>,
}

impl super::Command for Graph {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let rs = &rs;
        let range = format!(
            "{}..{}",
            rs.metadata().target,
            rs.metadata().steps.last().unwrap().cut
        );

        Ok(Box::new(GraphResult {
            range,
            git_args: self.git_args.clone(),
        }))
    }
}

pub struct GraphResult {
    pub range: String,
    pub git_args: Vec<String>,
}

impl PresentationOutput for GraphResult {
    fn present(&self, format: OutputFormat, repo: &GitRepo) -> Result<()> {
        let mut args = vec!["log", "--graph", "--oneline"];
        args.push(&self.range);
        for arg in &self.git_args {
            args.push(arg);
        }
        match format {
            OutputFormat::Human => {
                repo.run_interactive(&args)?;
                Ok(())
            }
            _ => {
                let output = repo.run(&args)?;
                PlainOutput(output).present(format, repo)
            }
        }
    }
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(
            &serde_json::json!({"graph": "Run in porcelain/json to see output"}),
        )?)
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
            let mut args = vec!["log", "--graph", "--oneline"];
            args.push(&range);
            for arg in &git_args {
                args.push(arg);
            }
            repo.run_interactive(&args)?;
            Ok(())
        }
        _ => {
            let mut args = vec!["log", "--graph", "--oneline"];
            args.push(&range);
            for arg in &git_args {
                args.push(arg);
            }
            let output = repo.run(&args)?;
            super::dispatch(format, repo, Ok(Box::new(PlainOutput(output))))
        }
    }
}

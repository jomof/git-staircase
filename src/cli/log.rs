use super::{
    CommitInfo, LogOutput, PresentationOutput, ResolvedSelector, StaircaseCommand,
    StaircaseSelectorArgs,
};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Log {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Log {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        super::run_staircase(self, repo)
    }
}

impl StaircaseCommand for Log {
    fn selector(&self) -> &StaircaseSelectorArgs {
        &self.staircase
    }

    fn run_resolved(
        &self,
        repo: &GitRepo,
        rs: &ResolvedSelector,
    ) -> Result<Box<dyn PresentationOutput>> {
        let m = rs.metadata();
        let target = &m.target;
        let tip = &m.steps.last().expect("Staircase has no steps").cut;

        let output = repo.run(&["log", "--oneline", &format!("{}..{}", target, tip)])?;

        let mut commits = Vec::new();
        for line in output.lines() {
            if let Some((hash, subject)) = line.split_once(' ') {
                commits.push(CommitInfo {
                    hash: hash.to_string(),
                    subject: subject.to_string(),
                });
            }
        }

        Ok(Box::new(LogOutput(commits)))
    }
}

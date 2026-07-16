use super::{CommitInfo, PresentationOutput, StaircaseCommits, StaircaseSelectorArgs, StepCommits};
use crate::GitRepo;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Commits {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
}

impl super::Command for Commits {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let target_oid = repo.resolve_commit(&rs.metadata().target)?;
        let mut current_base = target_oid;
        let mut steps = Vec::new();

        for (i, step) in rs.metadata().steps.iter().enumerate() {
            let mut step_commits = Vec::new();
            let commits_raw = repo.run(&[
                "log",
                "--oneline",
                &format!("{}..{}", current_base, step.cut),
            ])?;

            for line in commits_raw.lines() {
                if let Some((hash, subject)) = line.split_once(' ') {
                    step_commits.push(CommitInfo {
                        hash: hash.to_string(),
                        subject: subject.to_string(),
                    });
                }
            }

            steps.push(StepCommits {
                index: i + 1,
                name: step.name.clone(),
                commits: step_commits,
            });
            current_base = step.cut.clone();
        }

        Ok(Box::new(StaircaseCommits { steps }))
    }

    fn requires_clear_operation(&self) -> bool {
        false
    }
}

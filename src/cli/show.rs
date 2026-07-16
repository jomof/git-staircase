use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core::{persistence, refs::StaircaseRefs};
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Show {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub graph: bool,
    #[arg(long)]
    pub commits: bool,
    #[arg(long)]
    pub ids: bool,
    #[arg(long)]
    pub verification: bool,
}

impl super::Command for Show {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        if self.graph {
            let cmd = super::graph::Graph {
                staircase: self.staircase.clone(),
            };
            return cmd.run(repo);
        }
        if self.commits {
            let cmd = super::commits::Commits {
                staircase: self.staircase.clone(),
            };
            return cmd.run(repo);
        }
        if self
            .staircase
            .steps
            .as_ref()
            .map_or(false, |s| s.is_empty())
        {
            let cmd = super::steps::Steps {
                staircase: self.staircase.clone(),
            };
            return cmd.run(repo);
        }
        if self.ids {
            let cmd = super::id::Id {
                staircase: self.staircase.clone(),
                kind: crate::IdentityKind::Lineage,
            };
            return cmd.run(repo);
        }
        if self.verification {
            let cmd = super::verify::Verify {
                staircase: self.staircase.clone(),
                aggregate: false,
                each_prefix: false,
                profile: None,
                provider: None,
                build_command: None,
                test_command: None,
                draft: false,
            };
            return cmd.run(repo);
        }

        let rs = self.staircase.resolve(repo)?;
        if rs.is_managed() {
            let reference = StaircaseRefs::state_record(&rs.metadata().id);
            let record = persistence::read_record(repo, &reference)?;
            Ok(Box::new(record))
        } else {
            Ok(Box::new(rs.metadata().clone()))
        }
    }
}

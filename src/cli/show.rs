use super::{
    Command, PresentationOutput, ResolvedSelector, StaircaseCommand, StaircaseSelectorArgs,
};
use crate::GitRepo;
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
        // We handle delegation before resolution to avoid redundant work,
        // but since we want to consolidate boilerplate, we check flags first.
        if self.graph || self.commits || self.ids || self.verification {
            return self.delegate(repo);
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

        super::run_staircase(self, repo)
    }
}

impl Show {
    fn delegate(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
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
        unreachable!()
    }
}

impl StaircaseCommand for Show {
    fn selector(&self) -> &StaircaseSelectorArgs {
        &self.staircase
    }

    fn run_resolved(
        &self,
        _repo: &GitRepo,
        rs: &ResolvedSelector,
    ) -> Result<Box<dyn PresentationOutput>> {
        Ok(Box::new(rs.metadata().clone()))
    }
}

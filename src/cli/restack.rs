use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Restack {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
}

impl super::Command for Restack {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        if let Some(from) = &self.from {
            let token = from.rsplit_once(':').map(|(_, step)| step).unwrap_or(from);
            let index = token
                .parse::<usize>()
                .ok()
                .and_then(|ordinal| ordinal.checked_sub(1))
                .or_else(|| {
                    rs.metadata()
                        .steps
                        .iter()
                        .position(|step| step.id == token || step.name == token)
                })
                .ok_or_else(|| crate::StaircaseError::NotFound(from.clone()))?;
            core::restack_from(repo, &rs, index, self.dry_run)?;
        } else if !self.dry_run {
            core::restack(
                repo,
                &rs,
                core::RebaseOptions {
                    leave_upper_steps_stale: false,
                },
            )?;
        } else {
            core::restack_from(repo, &rs, 0, true)?;
        }
        Ok(Box::new(Success::new(format!(
            "Restacked staircase '{}'",
            rs.metadata().name
        ))))
    }
}

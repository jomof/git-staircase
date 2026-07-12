use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use crate::core::DropOptions;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Drop {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// Step number (1-based). Can be part of the staircase name (e.g. name:1)
    #[arg(long)]
    pub step: Option<usize>,
    #[arg(long)]
    pub restack: bool,
    #[arg(long)]
    pub no_restack: bool,
    #[arg(long)]
    pub leave_descendants_stale: bool,
}

impl super::Command for Drop {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let step_num = if let Some(s) = self.step {
            s
        } else {
            rs.step_index.map(|i| i + 1).ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?
        };

        if step_num == 0 {
            return Err(anyhow!("Step number must be 1-based"));
        }
        let step_index = step_num - 1;

        let restack = !self.no_restack && !self.leave_descendants_stale;
        core::drop(
            repo,
            &rs,
            step_index,
            DropOptions {
                restack,
                leave_descendants_stale: !restack,
            },
        )?;

        Ok(Box::new(Success::new(format!(
            "Dropped step {} from staircase '{}'",
            step_num,
            rs.metadata().name
        ))))
    }
}

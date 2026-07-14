use super::{PresentationOutput, StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Join {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// First step number (1-based). Can be part of the staircase name (e.g. name:1)
    #[arg(long)]
    pub step: Option<usize>,
    /// Second step number (1-based).
    #[arg(long)]
    pub step2: Option<usize>,
    /// Second step number if not using --step2.
    pub step2_pos: Option<String>,

    #[arg(long, conflicts_with = "keep_boundary_ref")]
    pub delete_boundary_ref: bool,
    #[arg(long, conflicts_with = "delete_boundary_ref")]
    pub keep_boundary_ref: bool,
    #[arg(long)]
    pub dry_run: bool,
}

impl super::Command for Join {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let ref_action = if self.delete_boundary_ref {
            core::JoinRefAction::Delete
        } else {
            core::JoinRefAction::Keep
        };

        let step_num1 = if let Some(s1) = self.step {
            s1
        } else {
            rs.step_index.map(|i| i + 1).ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?
        };

        let step_num2 = if let Some(s2) = self.step2 {
            s2
        } else if let Some(s2_str) = &self.step2_pos {
            if let Ok(n) = s2_str.parse::<usize>() {
                n
            } else {
                // Check if it's another step spec for the SAME staircase
                let (sc_name2, n) = crate::parse_step_spec(s2_str)?;
                if sc_name2 != rs.metadata().name && sc_name2 != rs.metadata().id {
                    return Err(anyhow!(
                        "Cannot join steps from different staircases: '{}' and '{}'",
                        rs.metadata().name,
                        sc_name2
                    ));
                }
                n
            }
        } else {
            return Err(anyhow!("Second step number must be provided"));
        };

        if step_num1 == 0 || step_num2 == 0 {
            return Err(anyhow!("Step numbers must be 1-based"));
        }

        if self.dry_run {
            let (low, high) = if step_num1 < step_num2 {
                (step_num1, step_num2)
            } else {
                (step_num2, step_num1)
            };
            if low + 1 != high || high > rs.metadata().steps.len() {
                return Err(anyhow!("join requires two adjacent steps"));
            }
        } else {
            core::join(
                repo,
                &rs.staircase,
                step_num1 - 1,
                step_num2 - 1,
                core::JoinOptions { ref_action },
            )?;
        }
        Ok(Box::new(Success::new(format!(
            "Joined steps {} and {} of staircase '{}'.",
            step_num1,
            step_num2,
            rs.metadata().name
        ))))
    }
}

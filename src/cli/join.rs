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
}

impl super::Command for Join {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(
            repo,
            self.staircase.clone(),
            self.step,
            self.step2,
            self.step2_pos.clone(),
        )?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    step2: Option<usize>,
    step2_pos: Option<String>,
) -> Result<Success> {
    let rs = staircase.resolve(repo)?;

    let step_num1 = if let Some(s1) = step {
        s1
    } else {
        rs.step_index.map(|i| i + 1).ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?
    };

    let step_num2 = if let Some(s2) = step2 {
        s2
    } else if let Some(s2_str) = step2_pos {
        if let Ok(n) = s2_str.parse::<usize>() {
            n
        } else {
            // Check if it's another step spec for the SAME staircase
            let (sc_name2, n) = crate::parse_step_spec(&s2_str)?;
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

    core::join(repo, &rs.staircase, step_num1 - 1, step_num2 - 1)?;
    Ok(Success::new(format!(
        "Joined steps {} and {} of staircase '{}'.",
        step_num1,
        step_num2,
        rs.metadata().name
    )))
}

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    step2: Option<usize>,
    step2_pos: Option<String>,
) -> Result<Success> {
    run_internal(repo, staircase, step, step2, step2_pos)
}

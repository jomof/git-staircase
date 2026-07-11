use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::anyhow;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    step2: Option<usize>,
    step2_pos: Option<String>,
) -> anyhow::Result<Success> {
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

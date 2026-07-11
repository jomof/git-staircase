use super::{StaircaseSelectorArgs, Success, resolve_rs};
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
    let (rs, step_num1, step_num2) = if let Some(s1) = step {
        let s2 = if let Some(s2) = step2 {
            s2
        } else if let Some(s2_str) = step2_pos {
            s2_str
                .parse::<usize>()
                .map_err(|e| anyhow!("Failed to parse step number '{}': {}", s2_str, e))?
        } else {
            return Err(anyhow!(
                "Second step number must be provided via --step2 or as a positional argument"
            ));
        };
        (resolve_rs(repo, &staircase)?, s1, s2)
    } else {
        let name_spec = staircase.name.as_ref().ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?;
        let (sc_name, s1) = crate::parse_step_spec(name_spec)?;

        let s2 = if let Some(s2) = step2 {
            s2
        } else if let Some(s2_str) = step2_pos {
            if let Ok(n) = s2_str.parse::<usize>() {
                n
            } else {
                let (sc_name2, n) = crate::parse_step_spec(&s2_str)?;
                if sc_name != sc_name2 {
                    return Err(anyhow!(
                        "Cannot join steps from different staircases: '{}' and '{}'",
                        sc_name,
                        sc_name2
                    ));
                }
                n
            }
        } else {
            return Err(anyhow!("Second step number must be provided"));
        };

        let mut sc_args = staircase.clone();
        sc_args.name = Some(sc_name);
        (resolve_rs(repo, &sc_args)?, s1, s2)
    };

    if step_num1 == 0 || step_num2 == 0 {
        return Err(anyhow!("Step numbers must be 1-based"));
    }

    core::join(repo, &rs, step_num1 - 1, step_num2 - 1)?;
    Ok(Success::new(format!(
        "Joined steps {} and {} of staircase '{}'.",
        step_num1,
        step_num2,
        rs.metadata().name
    )))
}

use super::{StaircaseSelectorArgs, Success};
use crate::GitRepo;
use crate::core;
use anyhow::anyhow;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    at: String,
    step_name: Option<String>,
) -> anyhow::Result<Success> {
    let (rs, step_num) = if let Some(s) = step {
        (staircase.resolve(repo)?, s)
    } else {
        let name_spec = staircase.name.as_ref().ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?;
        let (sc_name, step_num) = crate::parse_step_spec(name_spec)?;
        let mut sc_args = staircase.clone();
        sc_args.name = Some(sc_name);
        (sc_args.resolve(repo)?, step_num)
    };

    if step_num == 0 {
        return Err(anyhow!("Step number must be 1-based"));
    }
    core::split(repo, &rs, step_num - 1, &at, step_name.as_deref())?;
    Ok(Success::new(format!(
        "Split step {} of staircase '{}' at {}.",
        step_num,
        rs.metadata().name,
        at
    )))
}

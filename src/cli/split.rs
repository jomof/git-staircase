use super::{OutputFormat, StaircaseSelectorArgs, resolve_rs};
use crate::GitRepo;
use crate::core;
use anyhow::anyhow;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    step: Option<usize>,
    at: String,
    step_name: Option<String>,
) -> anyhow::Result<()> {
    let (rs, step_num) = if let Some(s) = step {
        (resolve_rs(repo, &staircase)?, s)
    } else {
        let name_spec = staircase.name.as_ref().ok_or_else(|| anyhow!("Step number must be provided either via --step or as part of the staircase name (e.g. name:1)"))?;
        let (sc_name, step_num) = crate::parse_step_spec(name_spec)?;
        let mut sc_args = staircase.clone();
        sc_args.name = Some(sc_name);
        (resolve_rs(repo, &sc_args)?, step_num)
    };

    if step_num == 0 {
        return Err(anyhow!("Step number must be 1-based"));
    }
    core::split(repo, &rs, step_num - 1, &at, step_name.as_deref())?;
    if matches!(format, OutputFormat::Human) {
        println!(
            "Split step {} of staircase '{}' at {}.",
            step_num,
            rs.metadata().name,
            at
        );
    }
    Ok(())
}

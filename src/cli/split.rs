use super::OutputFormat;
use crate::GitRepo;
use anyhow::anyhow;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    step: String,
    at: String,
    name: Option<String>,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let (sc_name, step_num) = super::super::parse_step_spec(&step)?;
    let rs = core::resolve_staircase(repo, &sc_name, onto.as_deref())?
        .ok_or_else(|| anyhow!("Staircase '{}' not found", sc_name))?;

    if step_num == 0 {
        return Err(anyhow!("Step number must be 1-based"));
    }
    core::split(repo, &rs, step_num - 1, &at, name.as_deref())?;
    if matches!(format, OutputFormat::Human) {
        println!(
            "Split step {} of staircase '{}' at {}.",
            step_num, sc_name, at
        );
    }
    Ok(())
}

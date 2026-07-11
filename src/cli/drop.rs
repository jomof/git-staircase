use crate::GitRepo;
use super::OutputFormat;
use anyhow::anyhow;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    step: String,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let (sc_name, step_num) = super::super::parse_step_spec(&step)?;
    let rs = core::resolve_staircase(repo, &sc_name, onto.as_deref())?
        .ok_or_else(|| anyhow!("Staircase '{}' not found", sc_name))?;
    core::drop(repo, &rs, step_num - 1)?;
    if matches!(format, OutputFormat::Human) {
        println!("Dropped step {} from staircase '{}'.", step_num, sc_name);
    }
    Ok(())
}

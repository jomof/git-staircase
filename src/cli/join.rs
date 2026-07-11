use super::OutputFormat;
use crate::GitRepo;
use crate::core;
use anyhow::anyhow;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    step1: String,
    step2: String,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let (sc_name1, step_num1) = crate::parse_step_spec(&step1)?;
    let (sc_name2, step_num2) = crate::parse_step_spec(&step2)?;

    if sc_name1 != sc_name2 {
        return Err(anyhow!(
            "Cannot join steps from different staircases: '{}' and '{}'",
            sc_name1,
            sc_name2
        ));
    }

    let rs = core::resolve_staircase(repo, &sc_name1, onto.as_deref())?
        .ok_or_else(|| anyhow!("Staircase '{}' not found", sc_name1))?;

    if step_num1 == 0 || step_num2 == 0 {
        return Err(anyhow!("Step numbers must be 1-based"));
    }

    core::join(repo, &rs, step_num1 - 1, step_num2 - 1)?;
    if matches!(format, OutputFormat::Human) {
        println!(
            "Joined steps {} and {} of staircase '{}'.",
            step_num1, step_num2, sc_name1
        );
    }
    Ok(())
}

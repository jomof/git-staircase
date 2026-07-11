use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&rs.metadata().steps)?);
        }
        _ => {
            for (i, step) in rs.metadata().steps.iter().enumerate() {
                println!("Step {}: {} ({})", i + 1, step.name, &step.cut[..7]);
            }
        }
    }
    Ok(())
}

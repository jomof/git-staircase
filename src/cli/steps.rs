use super::OutputFormat;
use crate::GitRepo;

pub fn run(
    repo: &GitRepo,
    _format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
    for (i, step) in rs.metadata().steps.iter().enumerate() {
        println!("Step {}: {} ({})", i + 1, step.name, &step.cut[..7]);
    }
    Ok(())
}

use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;

pub fn run(
    repo: &GitRepo,
    _format: OutputFormat,
    staircase: StaircaseSelectorArgs,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, &staircase)?;
    let target_oid = repo.resolve_ref(&rs.metadata().target)?;
    let mut current_base = target_oid;
    for (i, step) in rs.metadata().steps.iter().enumerate() {
        println!("Step {}: {}", i + 1, step.name);
        let commits = repo.run(&[
            "log",
            "--oneline",
            &format!("{}..{}", current_base, step.cut),
        ])?;
        for line in commits.lines() {
            println!("  {}", line);
        }
        current_base = step.cut.clone();
    }
    Ok(())
}

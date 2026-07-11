use crate::GitRepo;
use super::OutputFormat;

pub fn run(
    repo: &GitRepo,
    _format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
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

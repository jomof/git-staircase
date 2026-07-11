use super::OutputFormat;
use crate::GitRepo;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: String,
    resolve_onto: Option<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, resolve_onto)?;
    core::rebase(repo, &rs, &onto)?;
    if matches!(format, OutputFormat::Human) {
        println!("Rebased staircase onto '{}'.", onto);
    }
    Ok(())
}

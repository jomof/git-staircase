use crate::GitRepo;
use super::OutputFormat;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
    delete_branches: bool,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
    core::delete(repo, &rs.metadata().id, delete_branches)?;
    if matches!(format, OutputFormat::Human) {
        println!("Deleted staircase.");
    }
    Ok(())
}

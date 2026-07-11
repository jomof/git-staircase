use super::{OutputFormat, print_output};
use crate::GitRepo;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
    let status = core::get_status_metadata(repo, rs.metadata().clone())?;
    if matches!(format, OutputFormat::Human) && !rs.is_managed() {
        println!("(Implicit staircase)");
    }
    print_output(format, &status)
}

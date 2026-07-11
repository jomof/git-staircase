use super::{OutputFormat, print_output};
use crate::GitRepo;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
    print_output(format, &rs)
}

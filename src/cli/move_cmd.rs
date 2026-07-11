use super::OutputFormat;
use crate::GitRepo;
use anyhow::anyhow;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    from: usize,
    to: usize,
    onto: Option<String>,
    commits: Vec<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
    if from == 0 || to == 0 {
        return Err(anyhow!("Step numbers must be 1-based"));
    }
    core::move_commits(repo, &rs, from - 1, to - 1, &commits)?;
    if matches!(format, OutputFormat::Human) {
        println!("Moved commits.");
    }
    Ok(())
}

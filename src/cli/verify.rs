use super::{OutputFormat, print_output};
use crate::GitRepo;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
    aggregate: bool,
    each_prefix: bool,
    build_command: Option<String>,
    test_command: Option<String>,
) -> anyhow::Result<()> {
    let aggregate_opt = if aggregate { Some(true) } else { None };
    let each_prefix_opt = if each_prefix { Some(true) } else { None };

    let rs = super::resolve_rs(repo, name, steps, onto)?;

    let results = core::verify(
        repo,
        &rs,
        build_command,
        test_command,
        aggregate_opt,
        each_prefix_opt,
    )?;

    print_output(format, &results)
}

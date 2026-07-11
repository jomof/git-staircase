use crate::GitRepo;
use super::{OutputFormat, print_output};
use git_staircase::core;
use git_staircase::{Discovery, ResolvedStaircase};

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    managed: bool,
    implicit: bool,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let show_all = !managed && !implicit;
    let mut all_results = Vec::new();

    if managed || show_all {
        let list = repo.list_staircases()?;
        for s in list {
            all_results.push(ResolvedStaircase::Managed(s));
        }
    }

    if implicit || show_all {
        let list = core::discover(repo, onto.as_deref())?;
        for d in list {
            if let Discovery::Linear(s) = d {
                all_results.push(ResolvedStaircase::Implicit(s));
            }
        }
    }

    if matches!(format, OutputFormat::Human) {
        if all_results.is_empty() {
            println!("No staircases found.");
        } else {
            for r in all_results {
                let m = r.metadata();
                let status = core::get_status_metadata(repo, m.clone())?;
                let state = if status.steps.iter().any(|s| s.is_stale) {
                    "stale"
                } else {
                    "clean"
                };
                let steps_count = m.steps.len();
                let steps_word = if steps_count == 1 { "step" } else { "steps" };
                let implicit_marker = if r.is_managed() { "" } else { " (implicit)" };
                println!(
                    "{} {} {} {}{}",
                    m.name, steps_count, steps_word, state, implicit_marker
                );
            }
        }
        Ok(())
    } else {
        print_output(format, &all_results)
    }
}

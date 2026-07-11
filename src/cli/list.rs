use super::{OutputFormat, print_output};
use crate::GitRepo;
use crate::core;
use crate::core::persistence;
use crate::{Discovery, ResolvedStaircase};

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    managed: bool,
    implicit: bool,
    discovered: bool,
    families: bool,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let show_implicit = implicit || discovered;
    let show_all = !managed && !show_implicit && !families;
    let mut all_results = Vec::new();

    if managed || show_all {
        let list = persistence::list_staircases(repo)?;
        for s in list {
            all_results.push(ResolvedStaircase::Managed(s));
        }
    }

    if show_implicit || families || show_all {
        let list = core::discover(repo, onto.as_deref())?;
        for d in list {
            match d {
                Discovery::Linear(s) => {
                    if show_implicit || show_all {
                        all_results.push(ResolvedStaircase::Implicit(s));
                    }
                }
                Discovery::Ambiguous(f) => {
                    if families || show_all {
                        all_results.push(ResolvedStaircase::ImplicitFamily(f));
                    }
                }
            }
        }
    }

    if matches!(format, OutputFormat::Human) {
        if all_results.is_empty() {
            println!("No staircases found.");
        } else {
            for r in all_results {
                match r {
                    ResolvedStaircase::ImplicitFamily(f) => {
                        let path_count = f.steps.values().filter(|s| s.children.is_empty()).count();
                        let paths_word = if path_count == 1 { "path" } else { "paths" };
                        println!("{} {} {} (implicit)", f.name, path_count, paths_word);
                    }
                    _ => {
                        let m = r.metadata();
                        let status = core::get_status_metadata(repo, m.clone(), !r.is_managed())?;
                        let state = status.state();
                        let steps_count = m.steps.len();
                        let steps_word = if steps_count == 1 { "step" } else { "steps" };
                        let implicit_marker = if r.is_managed() { "" } else { " (implicit)" };
                        println!(
                            "{} {} {} {}{}",
                            m.name, steps_count, steps_word, state, implicit_marker
                        );
                    }
                }
            }
        }
        Ok(())
    } else {
        print_output(format, &all_results)
    }
}

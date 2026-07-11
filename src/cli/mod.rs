use anyhow::anyhow;
use git_staircase::model::{ToHuman, ToPorcelain};
use git_staircase::{GitRepo, core, ResolvedStaircase};
use serde::Serialize;

pub mod adopt;
pub mod commits;
pub mod delete;
pub mod diff;
pub mod discover;
pub mod drop;
pub mod graph;
pub mod id;
pub mod join;
pub mod list;
pub mod log;
pub mod move_cmd;
pub mod rebase;
pub mod reorder;
pub mod restack;
pub mod show;
pub mod split;
pub mod status;
pub mod steps;
pub mod verify;

pub enum OutputFormat {
    Human,
    Json,
    Porcelain,
}

pub fn print_output<T>(format: OutputFormat, value: &T) -> anyhow::Result<()>
where
    T: Serialize + ToHuman + ToPorcelain,
{
    match format {
        OutputFormat::Human => {
            let human = value.to_human();
            if !human.is_empty() {
                print!("{}", human);
                if !human.ends_with('\n') {
                    println!();
                }
            }
            Ok(())
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
            Ok(())
        }
        OutputFormat::Porcelain => {
            let porcelain = value.to_porcelain();
            if !porcelain.is_empty() {
                println!("{}", porcelain);
            }
            Ok(())
        }
    }
}

pub fn resolve_rs(
    repo: &GitRepo,
    name: Option<String>,
    steps: Option<Vec<String>>,
    onto: Option<String>,
) -> anyhow::Result<ResolvedStaircase> {
    if let Some(s) = steps {
        Ok(core::resolve_explicit_staircase(repo, &s, onto.as_deref())?)
    } else {
        let name = name.ok_or_else(|| anyhow!("Either a name or --steps must be provided"))?;
        core::resolve_staircase(repo, &name, onto.as_deref())?
            .ok_or_else(|| anyhow!("Staircase '{}' not found", name))
    }
}

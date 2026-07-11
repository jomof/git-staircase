use anyhow::{anyhow, Result};
use clap::Args;
use git_staircase::model::{ToHuman, ToPorcelain};
use git_staircase::{GitRepo, ResolvedStaircase, core};
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

#[derive(Args, Clone, Debug)]
pub struct StaircaseSelectorArgs {
    /// Name of the staircase to operate on. Can also be a branch name within a staircase.
    pub name: Option<String>,
    /// Explicit list of branch names (root to tip) for an unmanaged staircase.
    #[arg(long, value_delimiter = ',')]
    pub steps: Option<Vec<String>>,
    /// The target branch the staircase is based on (e.g., 'main').
    #[arg(long)]
    pub onto: Option<String>,
}

pub fn print_output<T>(format: OutputFormat, value: &T) -> Result<()>
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
    args: &StaircaseSelectorArgs,
) -> Result<ResolvedStaircase> {
    if let Some(s) = &args.steps {
        Ok(core::resolve_explicit_staircase(repo, s, args.onto.as_deref())?)
    } else {
        let name = args.name.as_ref().ok_or_else(|| anyhow!("Either a name or --steps must be provided"))?;
        core::resolve_staircase(repo, name, args.onto.as_deref())?
            .ok_or_else(|| anyhow!("Staircase '{}' not found", name))
    }
}


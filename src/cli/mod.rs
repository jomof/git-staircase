use crate::{GitRepo, ResolvedStaircase, core};
use anyhow::{Result, anyhow};
use clap::Args;
use serde::Serialize;

pub mod adopt;
pub mod commits;
pub mod delete;
pub mod diff;
pub mod discover;
pub mod drop;
pub mod formatting;
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

pub use formatting::{
    CommitInfo, LogOutput, PlainOutput, ReorderResult, StaircaseCommits, StepCommits, StepsList,
    Success, Summary, ToHuman, ToPorcelain,
};

#[derive(Clone, Copy, Debug)]
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
    /// Select by lineage ID.
    #[arg(long)]
    pub id: Option<String>,
    /// Select by staircase revision OID.
    #[arg(long)]
    pub revision: Option<String>,
    /// Select by managed staircase name.
    #[arg(long("name"))]
    pub explicit_name: Option<String>,
    /// Select by full staircase refname.
    #[arg(long("ref"))]
    pub r#ref: Option<String>,
    /// Select by structural revision key.
    #[arg(long)]
    pub structural_key: Option<String>,
}

impl StaircaseSelectorArgs {
    pub fn resolve(&self, repo: &GitRepo) -> Result<ResolvedStaircase> {
        if let Some(id) = &self.id {
            return Ok(core::resolve_by_id(repo, id)?);
        }
        if let Some(revision) = &self.revision {
            return Ok(core::resolve_by_revision(repo, revision)?);
        }
        if let Some(name) = &self.explicit_name {
            return Ok(core::resolve_by_name(repo, name)?);
        }
        if let Some(r) = &self.r#ref {
            return Ok(core::resolve_by_ref(repo, r)?);
        }
        if let Some(key) = &self.structural_key {
            return Ok(core::resolve_by_structural_key(
                repo,
                key,
                self.onto.as_deref(),
            )?);
        }

        if let Some(s) = &self.steps {
            Ok(core::resolve_explicit_staircase(
                repo,
                s,
                self.onto.as_deref(),
            )?)
        } else {
            let name = self
                .name
                .as_ref()
                .ok_or_else(|| anyhow!("Either a name, --steps, or an explicit selector (--id, --name, --ref, --revision, --structural-key) must be provided"))?;
            core::resolve_staircase(repo, name, self.onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))
        }
    }
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

pub fn dispatch<T>(format: OutputFormat, result: Result<T>) -> Result<()>
where
    T: Serialize + ToHuman + ToPorcelain,
{
    match result {
        Ok(val) => print_output(format, &val),
        Err(e) => Err(e),
    }
}

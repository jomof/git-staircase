use crate::{GitRepo, ResolvedSelector, core};
use anyhow::{Result, anyhow};
use clap::Args;
use serde::Serialize;

pub mod adopt;
pub mod commits;
pub mod delete;
pub mod diff;
pub mod discover;
pub mod draft;
pub mod drop;
pub mod formatting;
pub mod graph;
pub mod id;
pub mod join;
pub mod land;
pub mod list;
pub mod log;
pub mod move_cmd;
pub mod rebase;
pub mod reorder;
pub mod restack;
pub mod review;
pub mod show;
pub mod split;
pub mod status;
pub mod steps;
pub mod verify;
pub mod workspace;

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
    #[arg(long, value_delimiter = ',', num_args = 0..)]
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
    pub fn resolve(&self, repo: &GitRepo) -> Result<ResolvedSelector> {
        if let Some(id) = &self.id {
            return Ok(ResolvedSelector {
                staircase: core::resolve_by_id(repo, id)?,
                step_index: None,
            });
        }
        if let Some(revision) = &self.revision {
            return Ok(ResolvedSelector {
                staircase: core::resolve_by_revision(repo, revision)?,
                step_index: None,
            });
        }
        if let Some(name) = &self.explicit_name {
            return Ok(ResolvedSelector {
                staircase: core::resolve_by_name(repo, name)?,
                step_index: None,
            });
        }
        if let Some(r) = &self.r#ref {
            return Ok(ResolvedSelector {
                staircase: core::resolve_by_ref(repo, r)?,
                step_index: None,
            });
        }
        if let Some(key) = &self.structural_key {
            return Ok(ResolvedSelector {
                staircase: core::resolve_by_structural_key(repo, key, self.onto.as_deref())?,
                step_index: None,
            });
        }

        if let Some(s) = &self.steps {
            if !s.is_empty() {
                return Ok(ResolvedSelector {
                    staircase: core::resolve_explicit_staircase(repo, s, self.onto.as_deref())?,
                    step_index: None,
                });
            }
        }
        let name = self
            .name
            .as_ref()
            .ok_or_else(|| anyhow!("Either a name, --steps, or an explicit selector (--id, --name, --ref, --revision, --structural-key) must be provided"))?;
        core::resolve_staircase(repo, name, self.onto.as_deref())?
            .ok_or_else(|| anyhow!("Staircase '{}' not found", name))
    }
}

pub trait PresentationOutput {
    fn present(&self, format: OutputFormat, repo: &GitRepo) -> Result<()>;
    fn to_json(&self) -> Result<String>;
}

impl<T> PresentationOutput for T
where
    T: Serialize + ToHuman + ToPorcelain,
{
    fn present(&self, format: OutputFormat, _repo: &GitRepo) -> Result<()> {
        match format {
            OutputFormat::Human => {
                let human = self.to_human();
                if !human.is_empty() {
                    print!("{}", human);
                    if !human.ends_with('\n') {
                        println!();
                    }
                }
                Ok(())
            }
            OutputFormat::Json => {
                println!("{}", self.to_json()?);
                Ok(())
            }
            OutputFormat::Porcelain => {
                let porcelain = self.to_porcelain();
                if !porcelain.is_empty() {
                    println!("{}", porcelain);
                }
                Ok(())
            }
        }
    }
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

pub trait Command {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>>;
}

pub fn dispatch(
    format: OutputFormat,
    repo: &GitRepo,
    result: Result<Box<dyn PresentationOutput>>,
) -> Result<()> {
    match result {
        Ok(val) => val.present(format, repo),
        Err(e) => Err(e),
    }
}

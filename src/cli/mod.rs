use crate::{GitRepo, ResolvedSelector, StaircaseError, core};
use anyhow::{Result, anyhow};
use clap::Args;
use serde::Serialize;

pub mod adopt;
pub mod append;
pub mod archive;
pub mod commits;
pub mod delete;
pub mod describe;
pub mod diff;
pub mod discover;
pub mod discovery;
pub mod draft;
pub mod drop;
pub mod formatting;
pub mod graph;
pub mod id;
pub mod join;
pub mod land;
pub mod layout;
pub mod list;
pub mod log;
pub mod metadata;
pub mod monorepo;
pub mod move_cmd;
pub mod naming;
pub mod normalize;
pub mod operation;
pub mod policy;
pub mod provider;
pub mod rebase;
pub mod reorder;
pub mod restack;
pub mod rev_parse;
pub mod review;
pub mod show;
pub mod split;
pub mod status;
pub mod steps;
pub mod tag;
pub mod transport;
pub mod unarchive;
pub mod verify;
pub mod workspace;

pub use formatting::*;

#[derive(clap::ValueEnum, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Human,
    Json,
    Porcelain,
}

#[derive(Args, Clone, Debug)]
pub struct BaseStaircaseSelectorArgs {
    /// Name of the staircase to operate on. Can also be a branch name within a staircase.
    pub name: Option<String>,
    /// The target branch the staircase is based on (e.g., 'main').
    #[arg(long)]
    pub onto: Option<String>,
    /// Select by lineage ID.
    #[arg(long)]
    pub id: Option<String>,
    /// Select by exact record revision OID.
    #[arg(long)]
    pub record: Option<String>,
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

impl BaseStaircaseSelectorArgs {
    pub fn resolve(&self, repo: &GitRepo, steps: Option<&[String]>) -> Result<ResolvedSelector> {
        self.resolve_with_default(repo, steps, None)
    }

    pub fn resolve_with_default(
        &self,
        repo: &GitRepo,
        steps: Option<&[String]>,
        default: Option<&str>,
    ) -> Result<ResolvedSelector> {
        let selector_count = [
            self.id.is_some(),
            self.record.is_some(),
            self.explicit_name.is_some(),
            self.r#ref.is_some(),
            self.structural_key.is_some(),
            steps.as_ref().is_some_and(|steps| !steps.is_empty()),
            self.name.is_some(),
        ]
        .into_iter()
        .filter(|selected| *selected)
        .count();
        if selector_count > 1 {
            return Err(StaircaseError::Other(
                "exactly one staircase selector may be provided".into(),
            )
            .into());
        }
        if let Some(id) = &self.id {
            return Ok(ResolvedSelector {
                staircase: core::resolve_by_id(repo, id)?,
                step_index: None,
            });
        }
        if let Some(record) = &self.record {
            return Ok(ResolvedSelector {
                staircase: core::resolve_by_record(repo, record)?,
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

        if let Some(s) = steps {
            if !s.is_empty() {
                return Ok(ResolvedSelector {
                    staircase: core::resolve_explicit_staircase(repo, s, self.onto.as_deref())?,
                    step_index: None,
                });
            }
        }
        let name = self.name.as_deref().or(default).ok_or_else(|| {
            anyhow!("Either a name, --steps, or an explicit selector (--id, --name, --ref, --record, --structural-key) must be provided")
        })?;
        core::resolve_staircase(repo, name, self.onto.as_deref())?
            .ok_or_else(|| StaircaseError::NotFound(name.to_string()).into())
    }
}

#[derive(Args, Clone, Debug)]
pub struct StaircaseSelectorArgs {
    #[command(flatten)]
    pub base: BaseStaircaseSelectorArgs,
    /// Explicit list of branch names (root to tip) for an unmanaged staircase.
    #[arg(long, value_delimiter = ',', num_args = 0..)]
    pub steps: Option<Vec<String>>,
}

impl StaircaseSelectorArgs {
    pub fn resolve(&self, repo: &GitRepo) -> Result<ResolvedSelector> {
        self.base.resolve(repo, self.steps.as_deref())
    }

    pub fn resolve_with_default(&self, repo: &GitRepo, default: &str) -> Result<ResolvedSelector> {
        self.base
            .resolve_with_default(repo, self.steps.as_deref(), Some(default))
    }
}

#[derive(Args, Clone, Debug)]
pub struct RequiredStaircaseSelector {
    /// Canonical staircase selector.
    pub selector: String,
}

impl RequiredStaircaseSelector {
    pub fn resolve(&self, repo: &GitRepo) -> Result<ResolvedSelector> {
        core::resolve_staircase(repo, &self.selector, None)?
            .ok_or_else(|| StaircaseError::NotFound(self.selector.clone()).into())
    }
}

#[derive(Args, Clone, Debug)]
pub struct NonStepsStaircaseSelectorArgs {
    #[command(flatten)]
    pub base: BaseStaircaseSelectorArgs,
}

impl NonStepsStaircaseSelectorArgs {
    pub fn resolve(&self, repo: &GitRepo) -> Result<ResolvedSelector> {
        self.base.resolve(repo, None)
    }
}

pub trait PresentationOutput {
    fn present(&self, format: OutputFormat, repo: &GitRepo) -> Result<()>;
}

impl<T> PresentationOutput for T
where
    T: ToPresentation + Serialize,
{
    fn present(&self, format: OutputFormat, _repo: &GitRepo) -> Result<()> {
        match format {
            OutputFormat::Human => {
                let p = self.to_presentation();
                let human = formatting::render_human(&p, 0).trim_end().to_string();
                if !human.is_empty() {
                    println!("{}", human);
                }
                Ok(())
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(self)?);
                Ok(())
            }
            OutputFormat::Porcelain => {
                let p = self.to_presentation();
                let porcelain = formatting::render_porcelain(&p);
                if !porcelain.is_empty() {
                    print!("{}", porcelain);
                }
                Ok(())
            }
        }
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

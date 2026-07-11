use crate::model::{ToHuman, ToPorcelain};
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

pub fn resolve_rs(repo: &GitRepo, args: &StaircaseSelectorArgs) -> Result<ResolvedStaircase> {
    if let Some(id) = &args.id {
        return Ok(core::resolve_by_id(repo, id)?);
    }
    if let Some(revision) = &args.revision {
        return Ok(core::resolve_by_revision(repo, revision)?);
    }
    if let Some(name) = &args.explicit_name {
        return Ok(core::resolve_by_name(repo, name)?);
    }
    if let Some(r) = &args.r#ref {
        return Ok(core::resolve_by_ref(repo, r)?);
    }
    if let Some(key) = &args.structural_key {
        return Ok(core::resolve_by_structural_key(
            repo,
            key,
            args.onto.as_deref(),
        )?);
    }

    if let Some(s) = &args.steps {
        Ok(core::resolve_explicit_staircase(
            repo,
            s,
            args.onto.as_deref(),
        )?)
    } else {
        let name = args
            .name
            .as_ref()
            .ok_or_else(|| anyhow!("Either a name, --steps, or an explicit selector (--id, --name, --ref, --revision, --structural-key) must be provided"))?;
        core::resolve_staircase(repo, name, args.onto.as_deref())?
            .ok_or_else(|| anyhow!("Staircase '{}' not found", name))
    }
}

#[derive(Serialize)]
pub struct Success {
    pub message: String,
}

impl Success {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ToHuman for Success {
    fn to_human(&self) -> String {
        self.message.clone()
    }
}

impl ToPorcelain for Success {
    fn to_porcelain(&self) -> String {
        String::new()
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct Summary<T>(pub T);

impl ToHuman for Summary<crate::model::StaircaseStatus> {
    fn to_human(&self) -> String {
        let s = &self.0;
        let m = &s.metadata;
        let steps_count = m.steps.len();
        let steps_word = if steps_count == 1 { "step" } else { "steps" };
        let implicit_marker = if s.is_implicit { " (implicit)" } else { "" };
        format!(
            "{} {} {} {}{}",
            m.name,
            steps_count,
            steps_word,
            s.state(),
            implicit_marker
        )
    }
}

impl ToHuman for Summary<crate::model::StaircaseFamily> {
    fn to_human(&self) -> String {
        let f = &self.0;
        let path_count = f.steps.values().filter(|s| s.children.is_empty()).count();
        let paths_word = if path_count == 1 { "path" } else { "paths" };
        format!("{} {} {} (implicit)", f.name, path_count, paths_word)
    }
}

impl ToPorcelain for Summary<crate::model::StaircaseStatus> {
    fn to_porcelain(&self) -> String {
        self.0.to_porcelain()
    }
}

impl ToPorcelain for Summary<crate::model::StaircaseFamily> {
    fn to_porcelain(&self) -> String {
        format!(
            "{}\t{}\tfamily\t{}",
            self.0.name,
            self.0.id,
            self.0.steps.len()
        )
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ReorderResult {
    pub status: crate::model::StaircaseStatus,
}

impl ToHuman for ReorderResult {
    fn to_human(&self) -> String {
        "Reordered staircase.".to_string()
    }
}

impl ToPorcelain for ReorderResult {
    fn to_porcelain(&self) -> String {
        String::new()
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct StepsList(pub Vec<crate::model::Step>);

impl ToHuman for StepsList {
    fn to_human(&self) -> String {
        self.0
            .iter()
            .enumerate()
            .map(|(i, step)| format!("Step {}: {} ({})", i + 1, step.name, &step.cut[..7]))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToPorcelain for StepsList {
    fn to_porcelain(&self) -> String {
        self.0
            .iter()
            .enumerate()
            .map(|(i, step)| format!("{}\t{}\t{}", i + 1, step.name, step.cut))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

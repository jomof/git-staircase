use crate::GitRepo;
use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs};
use crate::core;
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct LayoutCmd {
    #[command(subcommand)]
    pub command: LayoutSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum LayoutSubcommand {
    Show(LayoutSelector),
    Check(LayoutSelector),
    Set(LayoutSet),
    Normalize(LayoutMutate),
    Rename(LayoutRename),
    Branch(LayoutBranch),
    Unset(LayoutUnset),
}

#[derive(Args, Clone, Debug)]
pub struct LayoutSelector {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct LayoutSet {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long, default_value = "sequential")]
    pub primary_branches: String,
    #[arg(long)]
    pub base: Option<String>,
    #[arg(long)]
    pub infer_base: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct LayoutMutate {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct LayoutRename {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub base: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct LayoutBranch {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub step: Option<usize>,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct LayoutUnset {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub primary_branches: bool,
    #[arg(long)]
    pub dry_run: bool,
}

impl Command for LayoutCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            LayoutSubcommand::Show(args) | LayoutSubcommand::Check(args) => {
                let selector = args.selector.resolve(repo)?;
                Ok(Box::new(core::layout_state(repo, &selector)?))
            }
            LayoutSubcommand::Set(args) => {
                if args.primary_branches != "sequential" {
                    return Err(anyhow!("only --primary-branches=sequential is supported"));
                }
                let selector = args.selector.resolve(repo)?;
                let base = match (&args.base, args.infer_base.as_deref()) {
                    (Some(base), None) => base.clone(),
                    (None, Some("strip-numeric-suffix")) => selector
                        .metadata()
                        .steps
                        .last()
                        .and_then(|step| step.branch.as_ref())
                        .map(|name| strip_numeric_suffix(name))
                        .ok_or_else(|| anyhow!("cannot infer layout base from a branchless tip"))?,
                    (None, Some(other)) => {
                        return Err(anyhow!("unsupported --infer-base value '{}'", other));
                    }
                    _ => return Err(anyhow!("provide exactly one of --base or --infer-base")),
                };
                Ok(Box::new(core::set_layout(
                    repo,
                    &selector,
                    &base,
                    args.dry_run,
                )?))
            }
            LayoutSubcommand::Normalize(args) => {
                let selector = args.selector.resolve(repo)?;
                Ok(Box::new(core::normalize(repo, &selector, args.dry_run)?))
            }
            LayoutSubcommand::Rename(args) => {
                let selector = args.selector.resolve(repo)?;
                Ok(Box::new(core::set_layout(
                    repo,
                    &selector,
                    &args.base,
                    args.dry_run,
                )?))
            }
            LayoutSubcommand::Branch(args) => {
                let selector = args.selector.resolve(repo)?;
                let explicit_index = match args.step {
                    Some(0) => return Err(anyhow!("step numbers are 1-based")),
                    Some(step) => Some(step - 1),
                    None => None,
                };
                let index = explicit_index
                    .or(selector.step_index)
                    .ok_or_else(|| anyhow!("select a step with :<ordinal> or --step"))?;
                Ok(Box::new(core::assign_step_branch(
                    repo,
                    &selector,
                    index,
                    &args.name,
                    args.dry_run,
                )?))
            }
            LayoutSubcommand::Unset(args) => {
                if !args.primary_branches {
                    return Err(anyhow!("--primary-branches is required"));
                }
                let selector = args.selector.resolve(repo)?;
                Ok(Box::new(core::unset_layout(repo, &selector, args.dry_run)?))
            }
        }
    }
}

fn strip_numeric_suffix(value: &str) -> String {
    let Some((base, suffix)) = value.rsplit_once('-') else {
        return value.into();
    };
    if !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit()) {
        base.into()
    } else {
        value.into()
    }
}

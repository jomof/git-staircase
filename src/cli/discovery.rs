use crate::GitRepo;
use crate::cli::{Command, PresentationOutput, RequiredStaircaseSelector, StaircaseSelectorArgs};
use crate::core;
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct DiscoveryCmd {
    #[command(subcommand)]
    pub command: DiscoverySubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum DiscoverySubcommand {
    Show(SelectorOnly),
    IncludeRef(OverrideValue),
    ExcludeRef(OverrideValue),
    AddCut(OverrideValue),
    IgnoreCut(OverrideValue),
    Clear(ClearOverride),
}

#[derive(Args, Clone, Debug)]
pub struct SelectorOnly {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct OverrideValue {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    pub value: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct ClearOverride {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    pub override_id: String,
    #[arg(long)]
    pub dry_run: bool,
}

impl Command for DiscoveryCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            DiscoverySubcommand::Show(args) => {
                let selector = args.selector.resolve(repo)?;
                Ok(Box::new(core::discovery_overrides(repo, &selector)?))
            }
            DiscoverySubcommand::IncludeRef(args) => mutate(repo, args, "include-ref"),
            DiscoverySubcommand::ExcludeRef(args) => mutate(repo, args, "exclude-ref"),
            DiscoverySubcommand::AddCut(args) => mutate(repo, args, "add-cut"),
            DiscoverySubcommand::IgnoreCut(args) => mutate(repo, args, "ignore-cut"),
            DiscoverySubcommand::Clear(args) => {
                let selector = args.selector.resolve(repo)?;
                Ok(Box::new(core::clear_discovery_override(
                    repo,
                    &selector,
                    &args.override_id,
                    args.dry_run,
                )?))
            }
        }
    }
}

fn mutate(repo: &GitRepo, args: &OverrideValue, kind: &str) -> Result<Box<dyn PresentationOutput>> {
    let selector = args.selector.resolve(repo)?;
    Ok(Box::new(core::add_discovery_override(
        repo,
        &selector,
        kind,
        &args.value,
        args.dry_run,
    )?))
}

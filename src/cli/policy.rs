use crate::GitRepo;
use crate::cli::{
    Command, PresentationOutput, RequiredStaircaseSelector, StaircaseSelectorArgs, StructuredOutput,
};
use crate::core;
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct PolicyCmd {
    #[command(subcommand)]
    pub command: PolicySubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum PolicySubcommand {
    Show(PolicyShow),
    Set(PolicySet),
    Unset(PolicyUnset),
}

#[derive(Args, Clone, Debug)]
pub struct PolicyShow {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct PolicySet {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    #[arg(required = true)]
    pub assignments: Vec<String>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct PolicyUnset {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    #[arg(required = true)]
    pub keys: Vec<String>,
    #[arg(long)]
    pub dry_run: bool,
}

impl Command for PolicyCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            PolicySubcommand::Show(args) => {
                let selector = args.selector.resolve(repo)?;
                Ok(Box::new(StructuredOutput(core::policy_values(
                    repo, &selector,
                )?)))
            }
            PolicySubcommand::Set(args) => {
                let selector = args.selector.resolve(repo)?;
                let assignments = args
                    .assignments
                    .iter()
                    .map(|assignment| {
                        let (key, raw) = assignment.split_once('=').ok_or_else(|| {
                            anyhow!("policy assignment must have the form <key>=<value>")
                        })?;
                        let value = serde_json::from_str(raw)
                            .unwrap_or_else(|_| serde_json::Value::String(raw.into()));
                        Ok((key.to_string(), Some(value)))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Box::new(StructuredOutput(core::update_policies(
                    repo,
                    &selector,
                    &assignments,
                    args.dry_run,
                )?)))
            }
            PolicySubcommand::Unset(args) => {
                let selector = args.selector.resolve(repo)?;
                let assignments = args
                    .keys
                    .iter()
                    .map(|key| (key.clone(), None))
                    .collect::<Vec<_>>();
                Ok(Box::new(StructuredOutput(core::update_policies(
                    repo,
                    &selector,
                    &assignments,
                    args.dry_run,
                )?)))
            }
        }
    }
}

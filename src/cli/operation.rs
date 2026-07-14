use crate::GitRepo;
use crate::cli::{Command, PresentationOutput, StructuredOutput};
use crate::core;
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct Continue {}

#[derive(Args, Clone, Debug)]
pub struct Abort {}

#[derive(Args, Clone, Debug)]
pub struct OperationCmd {
    #[command(subcommand)]
    pub command: OperationSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum OperationSubcommand {
    Show,
}

impl Command for Continue {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        Ok(Box::new(StructuredOutput(core::continue_active(repo)?)))
    }
}

impl Command for Abort {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        Ok(Box::new(StructuredOutput(core::abort_active(repo)?)))
    }
}

impl Command for OperationCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match self.command {
            OperationSubcommand::Show => {
                let operation = core::active_operation(repo)?
                    .ok_or(crate::StaircaseError::NoActiveOperation)?;
                Ok(Box::new(StructuredOutput(operation)))
            }
        }
    }
}

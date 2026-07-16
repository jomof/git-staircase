use super::PresentationOutput;
use crate::cli::StaircaseSelectorArgs;
use crate::core::persistence::read_record;
use crate::model::StaircaseRecord;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::GerritProvider;
use crate::workspace::github_provider::GitHubProvider;
use crate::workspace::review_provider::{ReviewProvider, ReviewProviderInstance};
use crate::{GitRepo, ResolvedSelector};
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct ReviewCmd {
    /// Select the review provider explicitly.
    #[arg(long, global = true)]
    pub provider: Option<String>,
    #[command(subcommand)]
    pub command: ReviewSubcommands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ReviewSubcommands {
    /// Show review details for a staircase
    Show(ReviewShowCmd),
    /// Show review and verification status for a staircase
    Status(ReviewStatusCmd),
    /// Generate upload plan for a staircase
    Plan(ReviewPlanCmd),
    /// Prepare durable review identities
    Create(ReviewCreateCmd),
    /// Attach an existing provider review
    Attach(ReviewAssociationCmd),
    /// Detach an active provider review association
    Detach(ReviewAssociationCmd),
    /// Upload staircase changes to review provider
    Upload(ReviewUploadCmd),
    /// Reconcile server review state with local staircase
    Reconcile(ReviewReconcileCmd),
    /// Open review in browser
    Open(ReviewOpenCmd),
}

#[derive(Args, Clone, Debug)]
pub struct ReviewShowCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewStatusCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewPlanCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub mapping: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewUploadCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub destination: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewCreateCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub mapping: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewAssociationCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub review: String,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewReconcileCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct ReviewOpenCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

impl super::Command for ReviewCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let boot_res = bootstrap(repo, &BootstrapOptions::default())?;

        let providers: Vec<Box<dyn ReviewProvider>> = match self.provider.as_deref() {
            Some("gerrit") => vec![Box::new(GerritProvider)],
            Some("github") => vec![Box::new(GitHubProvider)],
            Some(provider) => {
                return Err(anyhow!(
                    "Unknown review provider '{}'; expected gerrit or github",
                    provider
                ));
            }
            None => {
                if let Some(binding) = boot_res
                    .record
                    .capability_bindings
                    .get(&crate::workspace::Capability::Review)
                {
                    match binding.provider.as_str() {
                        "github" => vec![Box::new(GitHubProvider)],
                        "gerrit" => vec![Box::new(GerritProvider)],
                        _ => vec![Box::new(GerritProvider), Box::new(GitHubProvider)],
                    }
                } else {
                    vec![Box::new(GerritProvider), Box::new(GitHubProvider)]
                }
            }
        };

        for provider in providers {
            let provider_name = provider.name();
            if let Some(instance) = provider.probe(repo, Some(&boot_res.record))? {
                return self.run_instance(repo, provider_name, instance.as_ref());
            }
        }

        Err(anyhow!(
            "No review provider route (Gerrit or GitHub) could be resolved. Please configure remote host or workspace review route."
        ))
    }
}

impl ReviewCmd {
    fn run_instance(
        &self,
        repo: &GitRepo,
        _provider_name: &str,
        instance: &dyn ReviewProviderInstance,
    ) -> Result<Box<dyn PresentationOutput>> {
        let (resolved, oids, record) = self.resolve_context(repo)?;
        match &self.command {
            ReviewSubcommands::Show(_) => {
                Ok(Box::new(instance.show(repo, &oids, record.as_ref())?))
            }
            ReviewSubcommands::Status(_) => {
                Ok(Box::new(instance.status(repo, &oids, record.as_ref())?))
            }
            ReviewSubcommands::Plan(cmd) => Ok(Box::new(instance.plan(
                repo,
                &oids,
                cmd.mapping.as_deref(),
                record.as_ref(),
            )?)),
            ReviewSubcommands::Create(cmd) => Ok(Box::new(instance.create(
                repo,
                &oids,
                cmd.mapping.as_deref(),
                record.as_ref(),
            )?)),
            ReviewSubcommands::Attach(cmd) => Ok(Box::new(instance.attach(
                repo,
                &oids,
                &cmd.review,
                record.as_ref(),
                resolved.step_index,
            )?)),
            ReviewSubcommands::Detach(cmd) => Ok(Box::new(instance.detach(
                repo,
                &oids,
                &cmd.review,
                record.as_ref(),
                resolved.step_index,
            )?)),
            ReviewSubcommands::Upload(cmd) => Ok(Box::new(instance.upload(
                repo,
                &oids,
                cmd.destination.as_deref(),
                record.as_ref(),
            )?)),
            ReviewSubcommands::Reconcile(_) => Ok(Box::new(instance.reconcile(
                repo,
                &oids,
                record.as_ref(),
            )?)),
            ReviewSubcommands::Open(_) => {
                Ok(Box::new(instance.open(repo, &oids, record.as_ref())?))
            }
        }
    }

    fn resolve_context(
        &self,
        repo: &GitRepo,
    ) -> Result<(ResolvedSelector, Vec<String>, Option<StaircaseRecord>)> {
        let selector = match &self.command {
            ReviewSubcommands::Show(cmd) => &cmd.selector,
            ReviewSubcommands::Status(cmd) => &cmd.selector,
            ReviewSubcommands::Plan(cmd) => &cmd.selector,
            ReviewSubcommands::Create(cmd) => &cmd.selector,
            ReviewSubcommands::Attach(cmd) => &cmd.selector,
            ReviewSubcommands::Detach(cmd) => &cmd.selector,
            ReviewSubcommands::Upload(cmd) => &cmd.selector,
            ReviewSubcommands::Reconcile(cmd) => &cmd.selector,
            ReviewSubcommands::Open(cmd) => &cmd.selector,
        };
        let resolved = selector.resolve(repo)?;
        let oids = resolved
            .metadata()
            .steps
            .iter()
            .map(|s| s.cut.clone())
            .collect();
        let record = if resolved.is_managed() {
            Some(managed_record(repo, &resolved)?)
        } else {
            None
        };
        Ok((resolved, oids, record))
    }
}

pub fn managed_record(repo: &GitRepo, selector: &ResolvedSelector) -> Result<StaircaseRecord> {
    let reference = format!("refs/staircase-state/{}/record", selector.metadata().id);
    Ok(read_record(repo, &reference)?)
}

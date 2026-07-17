use super::{Presentation, PresentationOutput, StaircaseSelectorArgs, ToPresentation};
use crate::GitRepo;
use crate::core;
use crate::model::VerificationResult;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::GerritProvider;
use crate::workspace::github_provider::GitHubProvider;
use crate::workspace::review_provider::ReviewProvider;
use anyhow::Result;
use serde::Serialize;

#[derive(clap::Args, Clone, Debug)]
pub struct Verify {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long)]
    pub aggregate: bool,
    #[arg(long)]
    pub each_prefix: bool,
    #[arg(long)]
    pub profile: Option<String>,
    /// Use provider verification for exact remote review revisions.
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub build_command: Option<String>,
    #[arg(long)]
    pub test_command: Option<String>,
    /// Verify the exact current stage-zero index tree.
    #[arg(long)]
    pub draft: bool,
    /// Maximum number of seconds to allow the verification process to run before aborting it.
    #[arg(long)]
    pub timeout: Option<u64>,
}

impl super::Command for Verify {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        if self.draft {
            let evidence = core::verify_draft(
                repo,
                self.build_command.clone(),
                self.test_command.clone(),
                self.timeout,
            )?;
            return Ok(Box::new(evidence));
        }
        let aggregate_opt = if self.aggregate { Some(true) } else { None };
        let each_prefix_opt = if self.each_prefix { Some(true) } else { None };

        let rs = self.staircase.resolve(repo)?;
        if let Some(provider) = &self.provider {
            let workspace = bootstrap(repo, &BootstrapOptions::default())?;
            let provider_impl: Box<dyn ReviewProvider> = match provider.as_str() {
                "gerrit" => Box::new(GerritProvider),
                "github" => Box::new(GitHubProvider),
                value => {
                    return Err(crate::StaircaseError::Other(format!(
                        "unknown verification provider '{}'",
                        value
                    ))
                    .into());
                }
            };
            let instance = provider_impl
                .probe(repo, Some(&workspace.record))?
                .ok_or_else(|| {
                    crate::StaircaseError::Other(format!(
                        "{} provider route is not ready",
                        provider
                    ))
                })?;
            let oids = rs
                .metadata()
                .steps
                .iter()
                .map(|step| step.cut.clone())
                .collect::<Vec<_>>();
            return Ok(Box::new(instance.verify_provider(repo, &oids, None)?));
        }
        let results = core::verify(
            repo,
            &rs,
            self.build_command.clone(),
            self.test_command.clone(),
            aggregate_opt,
            each_prefix_opt,
            self.timeout,
        )?;
        Ok(Box::new(VerificationResults(results)))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct VerificationResults(pub Vec<VerificationResult>);

impl ToPresentation for VerificationResults {
    fn to_presentation(&self) -> Presentation {
        self.0.to_presentation()
    }
}

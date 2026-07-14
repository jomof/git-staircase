use super::{PresentationOutput, StaircaseSelectorArgs, StructuredOutput, Success};
use crate::GitRepo;
use crate::core;
use crate::model::LandingPolicy;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::GerritProvider;
use crate::workspace::github_provider::GitHubProvider;
use crate::workspace::review_provider::ReviewProvider;
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct Land {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// Override landing policy.
    #[arg(long)]
    pub policy: Option<LandingPolicy>,
    #[arg(long)]
    pub aggregate: bool,
    #[arg(long)]
    pub stepwise: bool,
    #[arg(long, conflicts_with_all = ["aggregate", "stepwise", "policy"])]
    pub through: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    /// Land through the selected review provider.
    #[arg(long)]
    pub provider: Option<String>,
    /// Provider landing method: merge, rebase, or squash.
    #[arg(long)]
    pub method: Option<String>,
    /// Request the provider merge queue for aggregate landing.
    #[arg(long)]
    pub queue: bool,
}

impl super::Command for Land {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        if let Some(provider) = &self.provider {
            let workspace = bootstrap(repo, &BootstrapOptions::default())?;
            let provider_impl: Box<dyn ReviewProvider> = match provider.as_str() {
                "gerrit" => Box::new(GerritProvider),
                "github" => Box::new(GitHubProvider),
                value => {
                    return Err(crate::StaircaseError::Other(format!(
                        "unknown landing provider '{}'",
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
            let mode = if self.aggregate || self.queue {
                "aggregate"
            } else {
                "stepwise"
            };
            return Ok(Box::new(StructuredOutput(instance.land(
                repo,
                &oids,
                mode,
                self.method.as_deref(),
            )?)));
        }
        if let Some(through) = &self.through {
            let token = through
                .rsplit_once(':')
                .map(|(_, step)| step)
                .unwrap_or(through);
            let index = token
                .parse::<usize>()
                .ok()
                .and_then(|ordinal| ordinal.checked_sub(1))
                .or_else(|| {
                    rs.metadata()
                        .steps
                        .iter()
                        .position(|step| step.id == token || step.name == token)
                })
                .ok_or_else(|| crate::StaircaseError::NotFound(through.clone()))?;
            core::land_through(repo, &rs, index, self.dry_run)?;
            return Ok(Box::new(Success::new(format!(
                "{} landing prefix of staircase '{}'",
                if self.dry_run { "Planned" } else { "Landed" },
                rs.metadata().name
            ))));
        }
        let policy = if self.aggregate {
            Some(LandingPolicy::AggregateOnly)
        } else if self.stepwise {
            Some(LandingPolicy::Stepwise)
        } else {
            self.policy
        };
        if !self.dry_run {
            core::land(repo, &rs, core::LandOptions { policy })?;
        }
        Ok(Box::new(Success::new(format!(
            "Landed staircase '{}'",
            rs.metadata().name
        ))))
    }
}

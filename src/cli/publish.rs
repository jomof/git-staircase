use super::PresentationOutput;
use crate::GitRepo;
use crate::cli::{StaircaseSelectorArgs, review::managed_record};
use crate::core;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::GerritProvider;
use crate::workspace::github_provider::GitHubProvider;
use crate::workspace::review_provider::ReviewProvider;
use anyhow::{Result, anyhow};
use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct Publish {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    /// Select the review provider explicitly.
    #[arg(long)]
    pub provider: Option<String>,
}

impl super::Command for Publish {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let boot_res = bootstrap(repo, &BootstrapOptions::default())?;

        let mut resolved = self.selector.resolve(repo)?;

        // 1. Adopt if implicit
        if !resolved.is_managed() {
            let mut metadata = resolved.metadata().clone();
            // Ensure it has a name if we're adopting it
            if metadata.name.is_empty() {
                metadata.name = "default".to_string();
            }
            core::adopt(repo, &metadata)?;
            // Re-resolve to get managed selector
            resolved = self.selector.resolve(repo)?;
        }

        let metadata = resolved.metadata();
        let mut record = managed_record(repo, &resolved)?;

        // 2. Initialize review provider
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

        let mut instance = None;
        for provider in providers {
            if let Some(inst) = provider.probe(repo, Some(&boot_res.record))? {
                instance = Some(inst);
                break;
            }
        }

        let instance = instance.ok_or_else(|| {
            anyhow!("No review provider route (Gerrit or GitHub) could be resolved.")
        })?;

        let oids: Vec<String> = metadata.steps.iter().map(|s| s.cut.clone()).collect();

        // 3. Create reviews if needed
        let create_res = instance.create(repo, &oids, None, Some(&record))?;
        // Update record after create
        if create_res.record_after.is_some() {
            let record_ref = core::refs::StaircaseRefs::state_record(&metadata.id);
            record = core::persistence::read_record(repo, &record_ref)?;
        }

        // 4. Upload
        let upload_res = instance.upload(repo, &oids, None, Some(&record))?;

        Ok(Box::new(upload_res))
    }
}

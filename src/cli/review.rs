use super::PresentationOutput;
use super::formatting::{ToHuman, ToPorcelain};
use crate::cli::StaircaseSelectorArgs;
use crate::core::persistence::read_record;
use crate::model::StaircaseRecord;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::GerritProvider;
use crate::workspace::github_provider::GitHubProvider;
use crate::workspace::review_provider::{
    ReviewProvider, ReviewProviderInstance, UnifiedReviewMutation, UnifiedReviewOpen,
    UnifiedReviewPlan, UnifiedReviewReconcile, UnifiedReviewShow, UnifiedReviewStatus,
    UnifiedReviewUpload,
};
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

impl ReviewCmd {
    pub fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
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

fn managed_record(repo: &GitRepo, selector: &ResolvedSelector) -> Result<StaircaseRecord> {
    let reference = format!("refs/staircase-state/{}/record", selector.metadata().id);
    Ok(read_record(repo, &reference)?)
}

impl ToHuman for UnifiedReviewShow {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("{} Host: {}", self.provider_label, self.host));
        lines.push(format!("Project: {}", self.project));
        lines.push(format!("Destination Branch: {}", self.destination_branch));
        for (k, v) in &self.details {
            lines.push(format!("{}: {}", k, v));
        }
        lines.push("Commits:".to_string());
        for item in &self.items {
            lines.push(format!(
                "  {} {} [{}]",
                &item.oid[..7.min(item.oid.len())],
                item.title,
                item.detail
            ));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewShow {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("host\t{}", self.host));
        lines.push(format!("project\t{}", self.project));
        for item in &self.items {
            lines.push(format!("commit\t{}\t{}", item.oid, item.detail));
        }
        lines.join("\n")
    }
}

impl ToHuman for UnifiedReviewStatus {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "{} Review Status: {}",
            self.provider_label, self.status
        ));
        lines.push(format!("Host: {}", self.host));
        lines.push(format!("Project: {}", self.project));
        for (k, v) in &self.details {
            lines.push(format!("{}: {}", k, v));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewStatus {
    fn to_porcelain(&self) -> String {
        format!("status\t{}", self.status)
    }
}

impl ToHuman for UnifiedReviewPlan {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("{} Upload Plan:", self.provider_label));
        lines.push(format!("  Target Ref: {}", self.target));
        lines.push(format!("  Mapping Policy: {}", self.policy));
        lines.push("  Commits to push:".to_string());
        for item in &self.items {
            lines.push(format!(
                "    - {} {} ({})",
                &item.oid[..7.min(item.oid.len())],
                item.title,
                item.detail
            ));
        }
        if !self.warnings.is_empty() {
            lines.push("  Warnings:".to_string());
            for w in &self.warnings {
                lines.push(format!("    - {}", w));
            }
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewPlan {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("push_ref\t{}", self.target));
        lines.push(format!("mapping_policy\t{}", self.policy));
        for item in &self.items {
            lines.push(format!("commit\t{}\t{}", item.oid, item.detail));
        }
        lines.join("\n")
    }
}

impl ToHuman for UnifiedReviewUpload {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("{} Upload Complete:", self.provider_label));
        lines.push(format!("  {}", self.summary));
        for detail in &self.details {
            lines.push(format!("  {}", detail));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewUpload {
    fn to_porcelain(&self) -> String {
        format!("result\t{}", self.summary)
    }
}

impl ToHuman for UnifiedReviewReconcile {
    fn to_human(&self) -> String {
        format!("{} Reconcile Status: {}", self.provider_label, self.status)
    }
}

impl ToPorcelain for UnifiedReviewReconcile {
    fn to_porcelain(&self) -> String {
        format!("status\t{}", self.status)
    }
}

impl ToHuman for UnifiedReviewOpen {
    fn to_human(&self) -> String {
        format!("{} Review URL: {}", self.provider_label, self.url)
    }
}

impl ToPorcelain for UnifiedReviewOpen {
    fn to_porcelain(&self) -> String {
        format!("url\t{}", self.url)
    }
}

impl ToHuman for UnifiedReviewMutation {
    fn to_human(&self) -> String {
        let mut lines = vec![format!(
            "{} review {}: {} association(s)",
            self.provider_label, self.action, self.changed
        )];
        lines.extend(self.details.iter().map(|detail| format!("  {}", detail)));
        if let (Some(before), Some(after)) = (&self.record_before, &self.record_after) {
            lines.push(format!("record revision: {} -> {}", before, after));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for UnifiedReviewMutation {
    fn to_porcelain(&self) -> String {
        format!("{}\t{}\t{}", self.action, self.changed, self.provider_label)
    }
}

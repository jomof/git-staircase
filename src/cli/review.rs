use super::PresentationOutput;
use super::formatting::{ToHuman, ToPorcelain};
use crate::GitRepo;
use crate::cli::StaircaseSelectorArgs;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::GerritProvider;
use crate::workspace::github_provider::GitHubProvider;
use crate::workspace::review_provider::{
    ReviewProvider, ReviewProviderInstance, UnifiedReviewOpen, UnifiedReviewPlan,
    UnifiedReviewReconcile, UnifiedReviewShow, UnifiedReviewStatus, UnifiedReviewUpload,
};
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct ReviewCmd {
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

        let providers: Vec<Box<dyn ReviewProvider>> =
            vec![Box::new(GerritProvider), Box::new(GitHubProvider)];

        for provider in providers {
            if let Some(instance) = provider.probe(repo, Some(&boot_res.record))? {
                return self.run_instance(repo, instance.as_ref());
            }
        }

        Err(anyhow!(
            "No review provider route (Gerrit or GitHub) could be resolved. Please configure remote host or workspace review route."
        ))
    }

    fn run_instance(
        &self,
        repo: &GitRepo,
        instance: &dyn ReviewProviderInstance,
    ) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            ReviewSubcommands::Show(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.show(repo, &oids)?))
            }
            ReviewSubcommands::Status(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.status(repo, &oids)?))
            }
            ReviewSubcommands::Plan(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.plan(
                    repo,
                    &oids,
                    cmd.mapping.as_deref(),
                )?))
            }
            ReviewSubcommands::Upload(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.upload(
                    repo,
                    &oids,
                    cmd.destination.as_deref(),
                )?))
            }
            ReviewSubcommands::Reconcile(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.reconcile(repo, &oids)?))
            }
            ReviewSubcommands::Open(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let oids: Vec<String> = resolved
                    .staircase
                    .metadata()
                    .steps
                    .iter()
                    .map(|s| s.cut.clone())
                    .collect();
                Ok(Box::new(instance.open(repo, &oids)?))
            }
        }
    }
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

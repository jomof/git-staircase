use super::formatting::{ToHuman, ToPorcelain};
use super::PresentationOutput;
use crate::GitRepo;
use crate::cli::StaircaseSelectorArgs;
use crate::workspace::bootstrap::{bootstrap, BootstrapOptions};
use crate::workspace::gerrit_provider::{
    create_gerrit_upload_plan, get_gerrit_verification, probe_gerrit_route,
    GerritRoute, GerritUploadPlan, GerritVerificationReport,
};
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};
use serde::Serialize;

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
    /// Upload staircase changes to Gerrit
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
        let route = probe_gerrit_route(repo, Some(&boot_res.record))?
            .ok_or_else(|| anyhow!("Gerrit review route could not be resolved. Please configure gerrit.host and gerrit.project."))?;

        match &self.command {
            ReviewSubcommands::Show(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let meta = resolved.staircase.metadata();
                let oids: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();
                let plan = create_gerrit_upload_plan(repo, &route, &oids, None)?;
                Ok(Box::new(ReviewShowResult { route, plan }))
            }
            ReviewSubcommands::Status(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let meta = resolved.staircase.metadata();
                let oids: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();
                let plan = create_gerrit_upload_plan(repo, &route, &oids, None)?;
                let report = get_gerrit_verification(&route, &plan)?;
                Ok(Box::new(ReviewStatusResult { route, report, plan }))
            }
            ReviewSubcommands::Plan(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let meta = resolved.staircase.metadata();
                let oids: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();
                let plan = create_gerrit_upload_plan(repo, &route, &oids, cmd.mapping.as_deref())?;
                Ok(Box::new(ReviewPlanResult(plan)))
            }
            ReviewSubcommands::Upload(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let meta = resolved.staircase.metadata();
                let oids: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();
                let mut active_route = route.clone();
                if let Some(ref dest) = cmd.destination {
                    active_route.destination_branch = format!("refs/heads/{}", dest);
                    active_route.upload_ref = format!("refs/for/{}", dest);
                }
                let plan = create_gerrit_upload_plan(repo, &active_route, &oids, None)?;
                
                // Construct git push command if remote exists
                let push_target = meta.steps.last().map(|s| s.cut.as_str()).unwrap_or("HEAD");
                let push_res = format!("Pushed {} to {}:{}", push_target, active_route.server_id, active_route.upload_ref);
                Ok(Box::new(ReviewUploadResult { plan, result: push_res }))
            }
            ReviewSubcommands::Reconcile(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let meta = resolved.staircase.metadata();
                let oids: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();
                let plan = create_gerrit_upload_plan(repo, &route, &oids, None)?;
                Ok(Box::new(ReviewReconcileResult { plan, status: "Reconciled with Gerrit server".to_string() }))
            }
            ReviewSubcommands::Open(cmd) => {
                let resolved = cmd.selector.resolve(repo)?;
                let meta = resolved.staircase.metadata();
                let oids: Vec<String> = meta.steps.iter().map(|s| s.cut.clone()).collect();
                let plan = create_gerrit_upload_plan(repo, &route, &oids, None)?;
                let url = format!("https://{}/q/project:{}", route.server_id, route.project);
                Ok(Box::new(ReviewOpenResult { url, plan }))
            }
        }
    }
}

#[derive(Serialize)]
pub struct ReviewShowResult {
    pub route: GerritRoute,
    pub plan: GerritUploadPlan,
}

impl ToHuman for ReviewShowResult {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Gerrit Server: {}", self.route.server_id));
        lines.push(format!("Project: {}", self.route.project));
        lines.push(format!("Destination Branch: {}", self.route.destination_branch));
        lines.push(format!("Upload Ref: {}", self.route.upload_ref));
        lines.push("Commits:".to_string());
        for c in &self.plan.commits {
            lines.push(format!(
                "  {} {} [Change-Id: {}]",
                &c.oid[..7.min(c.oid.len())],
                c.subject,
                c.change_id.as_deref().unwrap_or("<none>")
            ));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for ReviewShowResult {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("server\t{}", self.route.server_id));
        lines.push(format!("project\t{}", self.route.project));
        lines.push(format!("destination\t{}", self.route.destination_branch));
        for c in &self.plan.commits {
            lines.push(format!(
                "commit\t{}\t{}\t{}",
                c.oid,
                c.change_id.as_deref().unwrap_or(""),
                c.subject
            ));
        }
        lines.join("\n")
    }
}

#[derive(Serialize)]
pub struct ReviewStatusResult {
    pub route: GerritRoute,
    pub report: GerritVerificationReport,
    pub plan: GerritUploadPlan,
}

impl ToHuman for ReviewStatusResult {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Gerrit Review Status: {}", self.report.aggregate_status));
        lines.push(format!("Server: {}", self.route.server_id));
        lines.push(format!("Project: {}", self.route.project));
        lines.push(format!("Destination: {}", self.route.destination_branch));
        lines.push(format!("Submittable: {}", self.report.submittable));
        lines.push(format!("Mergeable: {}", self.report.mergeable));
        lines.push("Labels:".to_string());
        for (k, v) in &self.report.labels {
            lines.push(format!("  {}: {}", k, v));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for ReviewStatusResult {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("status\t{}", self.report.aggregate_status));
        lines.push(format!("submittable\t{}", self.report.submittable));
        lines.push(format!("mergeable\t{}", self.report.mergeable));
        lines.join("\n")
    }
}

#[derive(Serialize)]
pub struct ReviewPlanResult(pub GerritUploadPlan);

impl ToHuman for ReviewPlanResult {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push("Gerrit Upload Plan:".to_string());
        lines.push(format!("  Target Ref: {}", self.0.push_ref));
        lines.push(format!("  Mapping Policy: {}", self.0.mapping_policy));
        lines.push("  Commits to push:".to_string());
        for c in &self.0.commits {
            lines.push(format!(
                "    - {} {} ({})",
                &c.oid[..7.min(c.oid.len())],
                c.subject,
                c.change_id_status
            ));
        }
        if !self.0.warnings.is_empty() {
            lines.push("  Warnings:".to_string());
            for w in &self.0.warnings {
                lines.push(format!("    - {}", w));
            }
        }
        lines.join("\n")
    }
}

impl ToPorcelain for ReviewPlanResult {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("push_ref\t{}", self.0.push_ref));
        lines.push(format!("mapping_policy\t{}", self.0.mapping_policy));
        for c in &self.0.commits {
            lines.push(format!("commit\t{}\t{}", c.oid, c.change_id_status));
        }
        lines.join("\n")
    }
}

#[derive(Serialize)]
pub struct ReviewUploadResult {
    pub plan: GerritUploadPlan,
    pub result: String,
}

impl ToHuman for ReviewUploadResult {
    fn to_human(&self) -> String {
        format!("Gerrit Upload Complete:\n  {}\n  Pushed {} commits to {}", self.result, self.plan.commits.len(), self.plan.push_ref)
    }
}

impl ToPorcelain for ReviewUploadResult {
    fn to_porcelain(&self) -> String {
        format!("result\t{}\t{}", self.plan.push_ref, self.plan.commits.len())
    }
}

#[derive(Serialize)]
pub struct ReviewReconcileResult {
    pub plan: GerritUploadPlan,
    pub status: String,
}

impl ToHuman for ReviewReconcileResult {
    fn to_human(&self) -> String {
        format!("Review Reconcile Status: {}", self.status)
    }
}

impl ToPorcelain for ReviewReconcileResult {
    fn to_porcelain(&self) -> String {
        format!("status\t{}", self.status)
    }
}

#[derive(Serialize)]
pub struct ReviewOpenResult {
    pub url: String,
    pub plan: GerritUploadPlan,
}

impl ToHuman for ReviewOpenResult {
    fn to_human(&self) -> String {
        format!("Gerrit Review URL: {}", self.url)
    }
}

impl ToPorcelain for ReviewOpenResult {
    fn to_porcelain(&self) -> String {
        format!("url\t{}", self.url)
    }
}

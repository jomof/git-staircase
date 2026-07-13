use super::PresentationOutput;
use super::formatting::{ToHuman, ToPorcelain};
use crate::GitRepo;
use crate::workspace::{
    BootstrapOptions, WorkspaceDoctorReport, WorkspaceRecord, bootstrap,
    discover_installed_providers, doctor, forget_workspace_record,
};
use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;

#[derive(Args, Clone, Debug)]
pub struct WorkspaceCmd {
    #[command(subcommand)]
    pub command: WorkspaceSubcommands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum WorkspaceSubcommands {
    /// Show current workspace configuration
    Show(ShowCmd),
    /// Run passive discovery without persisting configuration
    Discover(DiscoverCmd),
    /// List installed provider implementations
    Providers(ProvidersCmd),
    /// Re-run discovery and update eligible automatic bindings
    Refresh(RefreshCmd),
    /// Diagnostic analysis of current workspace and provider health
    Doctor(DoctorCmd),
    /// Explicitly configure capability bindings or provider profile
    Configure(ConfigureCmd),
    /// Forget local workspace configuration record
    Forget(ForgetCmd),
}

#[derive(Args, Clone, Debug)]
pub struct ShowCmd {}

#[derive(Args, Clone, Debug)]
pub struct DiscoverCmd {}

#[derive(Args, Clone, Debug)]
pub struct ProvidersCmd {}

#[derive(Args, Clone, Debug)]
pub struct RefreshCmd {}

#[derive(Args, Clone, Debug)]
pub struct DoctorCmd {}

#[derive(Args, Clone, Debug)]
pub struct ConfigureCmd {
    #[arg(long)]
    pub provider_profile: Option<String>,
    #[arg(long)]
    pub workspace_provider: Option<String>,
    #[arg(long)]
    pub review_provider: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ForgetCmd {
    #[arg(long)]
    pub workspace_id: Option<String>,
}

impl WorkspaceCmd {
    pub fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            WorkspaceSubcommands::Show(_) => {
                let options = BootstrapOptions {
                    no_bootstrap: true,
                    ..Default::default()
                };
                let res = bootstrap(repo, &options)?;
                Ok(Box::new(WorkspaceShowOutput(res.record)))
            }
            WorkspaceSubcommands::Discover(_) => {
                let options = BootstrapOptions {
                    no_configure: true,
                    ..Default::default()
                };
                let res = bootstrap(repo, &options)?;
                Ok(Box::new(WorkspaceShowOutput(res.record)))
            }
            WorkspaceSubcommands::Providers(_) => {
                let installed = discover_installed_providers()?;
                let names: Vec<String> = installed
                    .into_iter()
                    .map(|p| p.descriptor.name)
                    .chain(vec![
                        "github".to_string(),
                        "gerrit".to_string(),
                        "repo".to_string(),
                        "core.git".to_string(),
                    ])
                    .collect();
                Ok(Box::new(WorkspaceProvidersOutput(names)))
            }
            WorkspaceSubcommands::Refresh(_) => {
                if let Ok(Some(rec)) =
                    crate::workspace::storage::find_workspace_record_for_path(&repo.workdir)
                {
                    let _ = crate::workspace::storage::forget_workspace_record(&rec.workspace_id);
                }
                let options = BootstrapOptions::default();
                let res = bootstrap(repo, &options)?;
                Ok(Box::new(WorkspaceShowOutput(res.record)))
            }
            WorkspaceSubcommands::Doctor(_) => {
                let options = BootstrapOptions::default();
                let report = doctor(repo, &options)?;
                Ok(Box::new(WorkspaceDoctorOutput(report)))
            }
            WorkspaceSubcommands::Configure(cmd) => {
                let options = BootstrapOptions {
                    workspace_provider: cmd.workspace_provider.clone(),
                    review_provider: cmd.review_provider.clone(),
                    provider_profile: cmd.provider_profile.clone(),
                    ..Default::default()
                };
                let res = bootstrap(repo, &options)?;
                Ok(Box::new(WorkspaceShowOutput(res.record)))
            }
            WorkspaceSubcommands::Forget(cmd) => {
                let selector = cmd
                    .workspace_id
                    .as_deref()
                    .unwrap_or_else(|| repo.workdir.to_str().unwrap_or(""));
                let removed = forget_workspace_record(selector)?;
                let msg = if removed {
                    format!("Forgot workspace record for '{}'", selector)
                } else {
                    format!("No workspace record found matching '{}'", selector)
                };
                Ok(Box::new(WorkspaceMessageOutput(msg)))
            }
        }
    }
}

#[derive(Serialize)]
pub struct WorkspaceShowOutput(pub WorkspaceRecord);

impl ToHuman for WorkspaceShowOutput {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Workspace ID: {}", self.0.workspace_id));
        lines.push(format!("Root: {}", self.0.canonical_root.display()));
        if let Some(ref proj) = self.0.current_project_id {
            lines.push(format!("Current Project: {}", proj));
        }
        lines.push("Capability Bindings:".to_string());
        for (cap, b) in &self.0.capability_bindings {
            lines.push(format!("  {}: {} ({})", cap, b.provider, b.provenance));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for WorkspaceShowOutput {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("id\t{}", self.0.workspace_id));
        lines.push(format!("root\t{}", self.0.canonical_root.display()));
        for (cap, b) in &self.0.capability_bindings {
            lines.push(format!(
                "binding\t{}\t{}\t{}",
                cap, b.provider, b.provenance
            ));
        }
        lines.join("\n")
    }
}

#[derive(Serialize)]
pub struct WorkspaceProvidersOutput(pub Vec<String>);

impl ToHuman for WorkspaceProvidersOutput {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push("Installed Providers:".to_string());
        for p in &self.0 {
            lines.push(format!("  - {}", p));
        }
        lines.join("\n")
    }
}

impl ToPorcelain for WorkspaceProvidersOutput {
    fn to_porcelain(&self) -> String {
        self.0.join("\n")
    }
}

#[derive(Serialize)]
pub struct WorkspaceDoctorOutput(pub WorkspaceDoctorReport);

impl ToHuman for WorkspaceDoctorOutput {
    fn to_human(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Workspace Doctor Status: {}", self.0.status));
        lines.push(format!("Workspace ID: {}", self.0.workspace_id));
        lines.push(format!("Canonical Root: {}", self.0.canonical_root));
        lines.push("Bound Capabilities:".to_string());
        for (k, v) in &self.0.bound_capabilities {
            lines.push(format!("  {}: {}", k, v));
        }
        if !self.0.missing_capabilities.is_empty() {
            lines.push(format!(
                "Missing Capabilities: {}",
                self.0.missing_capabilities.join(", ")
            ));
        }
        if !self.0.diagnostics.is_empty() {
            lines.push("Diagnostics:".to_string());
            for d in &self.0.diagnostics {
                lines.push(format!("  - {}", d));
            }
        }
        lines.join("\n")
    }
}

impl ToPorcelain for WorkspaceDoctorOutput {
    fn to_porcelain(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("status\t{}", self.0.status));
        lines.push(format!("id\t{}", self.0.workspace_id));
        for (k, v) in &self.0.bound_capabilities {
            lines.push(format!("binding\t{}\t{}", k, v));
        }
        lines.join("\n")
    }
}

#[derive(Serialize)]
pub struct WorkspaceMessageOutput(pub String);

impl ToHuman for WorkspaceMessageOutput {
    fn to_human(&self) -> String {
        self.0.clone()
    }
}

impl ToPorcelain for WorkspaceMessageOutput {
    fn to_porcelain(&self) -> String {
        self.0.clone()
    }
}

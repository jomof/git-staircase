use super::{Presentation, PresentationOutput, ToPresentation};
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

impl super::Command for WorkspaceCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            WorkspaceSubcommands::Show(_) => {
                let options = BootstrapOptions::default();
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

impl ToPresentation for WorkspaceShowOutput {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![Presentation::Field {
            label: "Root".into(),
            value: self.0.canonical_root.display().to_string(),
        }];
        if let Some(ref proj) = self.0.current_project_id {
            h_children.push(Presentation::Field {
                label: "Current Project".into(),
                value: proj.clone(),
            });
        }
        let mut bindings = vec![];
        for (cap, b) in &self.0.capability_bindings {
            let readiness = self
                .0
                .capability_readiness
                .get(cap)
                .map(ToString::to_string)
                .unwrap_or_else(|| "unknown".into());
            bindings.push(Presentation::Field {
                label: cap.to_string(),
                value: format!("{} ({}, {})", b.provider, b.provenance, readiness),
            });
        }
        h_children.push(Presentation::Section {
            title: "Capability Bindings:".into(),
            children: bindings,
        });

        let mut p_records = vec![
            Presentation::Record(vec!["id".into(), self.0.workspace_id.clone()]),
            Presentation::Record(vec![
                "root".into(),
                self.0.canonical_root.display().to_string(),
            ]),
        ];
        for (cap, b) in &self.0.capability_bindings {
            p_records.push(Presentation::Record(vec![
                "binding".into(),
                cap.to_string(),
                b.provider.clone(),
                b.provenance.to_string(),
            ]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Workspace ID: {}", self.0.workspace_id),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_records))),
        ])
    }
}

#[derive(Serialize)]
pub struct WorkspaceProvidersOutput(pub Vec<String>);

impl ToPresentation for WorkspaceProvidersOutput {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: "Installed Providers:".into(),
                children: self
                    .0
                    .iter()
                    .map(|p| Presentation::Plain(format!("- {}", p)))
                    .collect(),
            })),
            Presentation::Porcelain(Box::new(Presentation::List(
                self.0
                    .iter()
                    .map(|p| Presentation::Plain(p.clone()))
                    .collect(),
            ))),
        ])
    }
}

#[derive(Serialize)]
pub struct WorkspaceDoctorOutput(pub WorkspaceDoctorReport);

impl ToPresentation for WorkspaceDoctorOutput {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field {
                label: "Workspace ID".into(),
                value: self.0.workspace_id.clone(),
            },
            Presentation::Field {
                label: "Canonical Root".into(),
                value: self.0.canonical_root.clone(),
            },
        ];
        let mut capabilities = vec![];
        for capability in &self.0.capabilities {
            capabilities.push(Presentation::Field {
                label: capability.capability.clone(),
                value: format!(
                    "{} [{}; {}]{}",
                    capability.provider.as_deref().unwrap_or("<unbound>"),
                    capability.provenance.as_deref().unwrap_or("no-provenance"),
                    capability.readiness,
                    capability
                        .evidence
                        .as_ref()
                        .map(|evidence| format!(" — {}", evidence))
                        .unwrap_or_default()
                ),
            });
        }
        h_children.push(Presentation::Section {
            title: "Bound Capabilities:".into(),
            children: capabilities,
        });
        if !self.0.missing_capabilities.is_empty() {
            h_children.push(Presentation::Field {
                label: "Missing Capabilities".into(),
                value: self.0.missing_capabilities.join(", "),
            });
        }
        if !self.0.diagnostics.is_empty() {
            h_children.push(Presentation::Section {
                title: "Diagnostics:".into(),
                children: self
                    .0
                    .diagnostics
                    .iter()
                    .map(|d| Presentation::Plain(format!("- {}", d)))
                    .collect(),
            });
        }

        let mut p_records = vec![
            Presentation::Record(vec!["status".into(), self.0.status.clone()]),
            Presentation::Record(vec!["id".into(), self.0.workspace_id.clone()]),
        ];
        for (k, v) in &self.0.bound_capabilities {
            p_records.push(Presentation::Record(vec![
                "binding".into(),
                k.clone(),
                v.clone(),
            ]));
        }
        for capability in &self.0.capabilities {
            p_records.push(Presentation::Record(vec![
                "capability".into(),
                capability.capability.clone(),
                capability.provider.as_deref().unwrap_or("").into(),
                capability.provenance.as_deref().unwrap_or("").into(),
                capability.readiness.to_string(),
            ]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Workspace Doctor Status: {}", self.0.status),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_records))),
        ])
    }
}

#[derive(Serialize)]
pub struct WorkspaceMessageOutput(pub String);

impl ToPresentation for WorkspaceMessageOutput {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(self.0.clone())
    }
}

use super::{Presentation, PresentationOutput, ToPresentation};
use crate::GitRepo;
use crate::workspace::gerrit_provider::probe_gerrit_route;
use crate::workspace::github_provider::probe_github_route;
use crate::workspace::repo_provider::observe_repo_workspace;
use crate::workspace::storage::find_workspace_record_for_path;
use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Args, Clone, Debug)]
pub struct ProviderCmd {
    #[command(subcommand)]
    pub provider: ProviderSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ProviderSubcommand {
    /// Diagnose the repo workspace provider
    Repo(ProviderAction),
    /// Diagnose the Gerrit review provider
    Gerrit(ProviderAction),
    /// Diagnose the GitHub review provider
    Github(ProviderAction),
}

#[derive(Args, Clone, Debug)]
pub struct ProviderAction {
    #[command(subcommand)]
    pub action: ProviderActionSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ProviderActionSubcommand {
    /// Run an offline, nonmutating provider diagnostic
    Doctor,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderDoctorReport {
    pub provider: String,
    pub applicable: bool,
    pub readiness: String,
    pub route: HashMap<String, String>,
    pub evidence: Vec<String>,
    pub diagnostics: Vec<String>,
    pub passive_network_requests: usize,
    pub workspace_mutations: usize,
}

impl ToPresentation for ProviderDoctorReport {
    fn to_presentation(&self) -> Presentation {
        let mut route_fields = vec![];
        let mut route_keys: Vec<_> = self.route.keys().collect();
        route_keys.sort();
        for key in route_keys {
            route_fields.push(Presentation::Field {
                label: key.clone(),
                value: self.route.get(key).unwrap().clone(),
            });
        }

        let mut h_children = vec![
            Presentation::Field {
                label: "Applicable".to_string(),
                value: self.applicable.to_string(),
            },
            Presentation::Field {
                label: "Readiness".to_string(),
                value: self.readiness.clone(),
            },
            Presentation::Section {
                title: "Route:".to_string(),
                children: route_fields,
            },
            Presentation::Plain(format!(
                "Passive effects: {} network request(s), {} workspace mutation(s)",
                self.passive_network_requests, self.workspace_mutations
            )),
        ];
        for d in &self.diagnostics {
            h_children.push(Presentation::Field {
                label: "Diagnostic".to_string(),
                value: d.clone(),
            });
        }

        let mut p_records = vec![
            Presentation::Record(vec!["provider".into(), self.provider.clone()]),
            Presentation::Record(vec!["applicable".into(), self.applicable.to_string()]),
            Presentation::Record(vec!["readiness".into(), self.readiness.clone()]),
        ];
        let mut route_keys: Vec<_> = self.route.keys().collect();
        route_keys.sort();
        for key in route_keys {
            p_records.push(Presentation::Record(vec![
                "route".into(),
                key.clone(),
                self.route.get(key).unwrap().clone(),
            ]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Provider: {}", self.provider),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_records))),
        ])
    }
}

impl crate::cli::Command for ProviderCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let record = find_workspace_record_for_path(&repo.workdir)?;
        let report = match &self.provider {
            ProviderSubcommand::Repo(_) => {
                if let Some(observation) = observe_repo_workspace(repo)? {
                    ProviderDoctorReport {
                        provider: "repo".into(),
                        applicable: true,
                        readiness: "ready".into(),
                        route: HashMap::from([
                            (
                                "workspace-root".into(),
                                observation.mapping.workspace_root.display().to_string(),
                            ),
                            ("project".into(), observation.mapping.project_name),
                            (
                                "project-path".into(),
                                observation.mapping.project_path.display().to_string(),
                            ),
                            (
                                "git-common-dir".into(),
                                observation.mapping.git_common_dir.display().to_string(),
                            ),
                        ]),
                        evidence: observation.candidate.evidence,
                        diagnostics: observation.diagnostics,
                        passive_network_requests: 0,
                        workspace_mutations: 0,
                    }
                } else {
                    unavailable("repo", "current repository is not uniquely mapped by repo")
                }
            }
            ProviderSubcommand::Gerrit(_) => {
                if let Some(route) = probe_gerrit_route(repo, record.as_ref())? {
                    ProviderDoctorReport {
                        provider: "gerrit".into(),
                        applicable: true,
                        readiness: "ready".into(),
                        route: HashMap::from([
                            ("server".into(), route.server_id),
                            ("project".into(), route.project),
                            ("destination".into(), route.destination_branch),
                            ("upload-ref".into(), route.upload_ref),
                        ]),
                        evidence: vec!["unique complete local Gerrit route".into()],
                        diagnostics: vec![
                            "authentication and remote permissions are not tested offline".into(),
                        ],
                        passive_network_requests: 0,
                        workspace_mutations: 0,
                    }
                } else {
                    unavailable("gerrit", "no complete unique local Gerrit route")
                }
            }
            ProviderSubcommand::Github(_) => {
                if let Some(route) = probe_github_route(repo, record.as_ref())? {
                    ProviderDoctorReport {
                        provider: "github".into(),
                        applicable: true,
                        readiness: "ready".into(),
                        route: HashMap::from([
                            ("installation".into(), route.installation),
                            ("base-repository".into(), route.base_repository.full_name()),
                            ("destination".into(), route.destination_branch),
                            ("remote".into(), route.remote_name),
                        ]),
                        evidence: vec!["canonical GitHub remote found locally".into()],
                        diagnostics: vec![
                            "authentication, default branch, and permissions are not guessed"
                                .into(),
                        ],
                        passive_network_requests: 0,
                        workspace_mutations: 0,
                    }
                } else {
                    unavailable("github", "no unique local GitHub repository route")
                }
            }
        };
        Ok(Box::new(report))
    }
}

fn unavailable(provider: &str, diagnostic: &str) -> ProviderDoctorReport {
    ProviderDoctorReport {
        provider: provider.into(),
        applicable: false,
        readiness: "unavailable".into(),
        route: HashMap::new(),
        evidence: Vec::new(),
        diagnostics: vec![diagnostic.into()],
        passive_network_requests: 0,
        workspace_mutations: 0,
    }
}

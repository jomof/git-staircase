use crate::error::Result;
use crate::git::GitRepo;
use crate::workspace::bootstrap::{bootstrap, BootstrapOptions};
use crate::workspace::model::Capability;
use crate::workspace::provider::discover_installed_providers;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceDoctorReport {
    pub workspace_id: String,
    pub canonical_root: String,
    pub current_project_id: Option<String>,
    pub bound_capabilities: HashMap<String, String>,
    pub missing_capabilities: Vec<String>,
    pub installed_providers: Vec<String>,
    pub status: String,
    pub diagnostics: Vec<String>,
}

pub fn doctor(repo: &GitRepo, options: &BootstrapOptions) -> Result<WorkspaceDoctorReport> {
    let bootstrap_res = bootstrap(repo, options)?;
    let rec = bootstrap_res.record;

    let installed = discover_installed_providers()?;
    let installed_names: Vec<String> = installed
        .into_iter()
        .map(|p| p.descriptor.name)
        .chain(std::iter::once("core.git".to_string()))
        .collect();

    let mut bound_map = HashMap::new();
    let all_capabilities = [
        Capability::Workspace,
        Capability::ProjectMapping,
        Capability::IntegrationContext,
        Capability::Review,
        Capability::Verification,
        Capability::Transport,
    ];

    let mut missing = Vec::new();
    for cap in &all_capabilities {
        if let Some(b) = rec.capability_bindings.get(cap) {
            bound_map.insert(cap.to_string(), b.provider.clone());
        } else {
            missing.push(cap.to_string());
        }
    }

    let mut diagnostics = Vec::new();

    if rec.capability_bindings.get(&Capability::Review).is_none() {
        diagnostics.push("Review provider is unbound. Commands requiring code review integration will fail.".to_string());
    }

    if rec
        .capability_bindings
        .get(&Capability::Verification)
        .is_none()
    {
        diagnostics.push("Verification provider is unbound.".to_string());
    }

    let status = if diagnostics.is_empty() {
        "Healthy".to_string()
    } else {
        "Degraded / Partial".to_string()
    };

    Ok(WorkspaceDoctorReport {
        workspace_id: rec.workspace_id,
        canonical_root: rec.canonical_root.to_string_lossy().to_string(),
        current_project_id: rec.current_project_id,
        bound_capabilities: bound_map,
        missing_capabilities: missing,
        installed_providers: installed_names,
        status,
        diagnostics,
    })
}

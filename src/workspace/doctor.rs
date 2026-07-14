use crate::error::Result;
use crate::git::GitRepo;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
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
    pub capabilities: Vec<CapabilityDoctorEntry>,
    pub discovery_fingerprint: HashMap<String, String>,
    pub last_successful_validation: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityDoctorEntry {
    pub capability: String,
    pub provider: Option<String>,
    pub provenance: Option<String>,
    pub readiness: String,
    pub evidence: Option<String>,
}

pub fn doctor(repo: &GitRepo, options: &BootstrapOptions) -> Result<WorkspaceDoctorReport> {
    let bootstrap_res = bootstrap(repo, options)?;
    let rec = bootstrap_res.record;

    let installed = discover_installed_providers()?;
    let mut installed_names: Vec<String> = installed
        .into_iter()
        .map(|p| p.descriptor.name)
        .chain(
            ["core.git", "repo", "gerrit", "github"]
                .into_iter()
                .map(str::to_string),
        )
        .collect();
    installed_names.sort();
    installed_names.dedup();

    let mut bound_map = HashMap::new();
    let mut missing = Vec::new();
    let mut capabilities = Vec::new();
    for cap in &Capability::ALL {
        if let Some(b) = rec.capability_bindings.get(cap) {
            bound_map.insert(cap.to_string(), b.provider.clone());
            capabilities.push(CapabilityDoctorEntry {
                capability: cap.to_string(),
                provider: Some(b.provider.clone()),
                provenance: Some(b.provenance.to_string()),
                readiness: rec
                    .capability_readiness
                    .get(cap)
                    .cloned()
                    .unwrap_or_default()
                    .to_string(),
                evidence: b.evidence.clone(),
            });
        } else {
            missing.push(cap.to_string());
            capabilities.push(CapabilityDoctorEntry {
                capability: cap.to_string(),
                provider: None,
                provenance: None,
                readiness: "unavailable".into(),
                evidence: None,
            });
        }
    }

    let mut diagnostics = Vec::new();

    if rec.capability_bindings.get(&Capability::Review).is_none() {
        diagnostics.push(
            "Review provider is unbound. Commands requiring code review integration will fail."
                .to_string(),
        );
    }

    if rec
        .capability_bindings
        .get(&Capability::Verification)
        .is_none()
    {
        diagnostics.push("Verification provider is unbound.".to_string());
    }

    for capability in &capabilities {
        if !matches!(capability.readiness.as_str(), "ready") {
            diagnostics.push(format!(
                "{} capability is {}{}",
                capability.capability,
                capability.readiness,
                capability
                    .provider
                    .as_ref()
                    .map(|provider| format!(" (provider {})", provider))
                    .unwrap_or_default()
            ));
        }
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
        capabilities,
        discovery_fingerprint: rec.discovery_fingerprint,
        last_successful_validation: rec.last_successful_validation,
        generation: rec.generation,
    })
}

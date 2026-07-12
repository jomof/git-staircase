use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::workspace::model::{
    BindingProvenance, Capability, CapabilityBinding, WorkspaceCandidate, WorkspaceRecord,
};
use crate::workspace::provider::{
    discover_installed_providers, expand_profile, get_core_git_candidate,
    invoke_provider_probe_workspace,
};
use crate::workspace::storage::{
    current_timestamp, find_workspace_record_for_path, load_workspace_record_by_id,
    save_workspace_record,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct BootstrapOptions {
    pub no_bootstrap: bool,
    pub no_configure: bool,
    pub workspace_id: Option<String>,
    pub workspace_provider: Option<String>,
    pub review_provider: Option<String>,
    pub provider_profile: Option<String>,
    pub workspace_mode: Option<String>,
    pub is_porcelain_or_json: bool,
}

#[derive(Debug, Clone)]
pub struct BootstrapResult {
    pub record: WorkspaceRecord,
    pub newly_configured: bool,
    pub message: Option<String>,
    pub warning: Option<String>,
}

pub fn bootstrap(repo: &GitRepo, options: &BootstrapOptions) -> Result<BootstrapResult> {
    // Phase 1: Establish Git context
    let workdir = repo
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone());

    if options.no_bootstrap {
        if let Some(existing) = find_workspace_record_for_path(&workdir)? {
            return Ok(BootstrapResult {
                record: existing,
                newly_configured: false,
                message: None,
                warning: None,
            });
        } else {
            let record = make_default_core_git_record(&workdir);
            return Ok(BootstrapResult {
                record,
                newly_configured: false,
                message: None,
                warning: None,
            });
        }
    }

    // Phase 2: Locate existing workspace configuration
    if let Some(ref ws_id) = options.workspace_id {
        if let Some(record) = load_workspace_record_by_id(ws_id)? {
            return Ok(BootstrapResult {
                record,
                newly_configured: false,
                message: None,
                warning: None,
            });
        }
    }

    let existing = find_workspace_record_for_path(&workdir)?;
    if let Some(mut record) = existing {
        if options.workspace_mode.as_deref() == Some("single-git") {
            record.capability_bindings.insert(
                Capability::Workspace,
                CapabilityBinding {
                    provider: "core.git".to_string(),
                    provenance: BindingProvenance::Explicit,
                    evidence: Some("explicit --workspace-mode=single-git".to_string()),
                },
            );
        }
        return Ok(BootstrapResult {
            record,
            newly_configured: false,
            message: None,
            warning: None,
        });
    }

    // Single-Git mode requested explicitly
    if options.workspace_mode.as_deref() == Some("single-git") {
        let record = make_default_core_git_record(&workdir);
        if !options.no_configure {
            let _ = save_workspace_record(&record);
        }
        return Ok(BootstrapResult {
            record,
            newly_configured: true,
            message: Some(format!(
                "Configured Staircase workspace:\n  workspace: single Git repository\n  root: {}",
                workdir.display()
            )),
            warning: None,
        });
    }

    // Phase 3: Probe workspace providers
    let installed = discover_installed_providers()?;
    let mut candidates = Vec::new();

    for prov in &installed {
        if let Ok(Some(cand)) = invoke_provider_probe_workspace(prov, repo) {
            candidates.push(cand);
        }
    }

    let fallback_cand = get_core_git_candidate(repo);
    candidates.push(fallback_cand);

    // Phase 4: Select workspace candidate
    let selected_candidate = select_workspace_candidate(&candidates, options)?;

    // Phase 5 & 6: Probe dependent capabilities & Bind capabilities
    let mut bindings = HashMap::new();
    let mut provenances = HashMap::new();

    let ws_provider_name = selected_candidate.provider.clone();

    bindings.insert(
        Capability::Workspace,
        CapabilityBinding {
            provider: ws_provider_name.clone(),
            provenance: if options.workspace_provider.is_some() {
                BindingProvenance::Explicit
            } else if ws_provider_name == "core.git" {
                BindingProvenance::Default
            } else {
                BindingProvenance::AutoDiscovered
            },
            evidence: selected_candidate.evidence.first().cloned(),
        },
    );
    provenances.insert(Capability::Workspace, BindingProvenance::AutoDiscovered);

    bindings.insert(
        Capability::ProjectMapping,
        CapabilityBinding {
            provider: ws_provider_name.clone(),
            provenance: BindingProvenance::AutoDiscovered,
            evidence: None,
        },
    );
    bindings.insert(
        Capability::IntegrationContext,
        CapabilityBinding {
            provider: ws_provider_name.clone(),
            provenance: BindingProvenance::AutoDiscovered,
            evidence: None,
        },
    );
    bindings.insert(
        Capability::Transport,
        CapabilityBinding {
            provider: "git".to_string(),
            provenance: BindingProvenance::Default,
            evidence: None,
        },
    );

    // Profile handling
    if let Some(ref prof) = options.provider_profile {
        let profile_bindings = expand_profile(prof);
        for (cap, prov) in profile_bindings {
            bindings.insert(
                cap,
                CapabilityBinding {
                    provider: prov,
                    provenance: BindingProvenance::Profile,
                    evidence: Some(format!("profile {}", prof)),
                },
            );
            provenances.insert(cap, BindingProvenance::Profile);
        }
    }

    // Explicit review provider override
    if let Some(ref rev_prov) = options.review_provider {
        bindings.insert(
            Capability::Review,
            CapabilityBinding {
                provider: rev_prov.clone(),
                provenance: BindingProvenance::Explicit,
                evidence: None,
            },
        );
        provenances.insert(Capability::Review, BindingProvenance::Explicit);
    }

    let workspace_id = uuid::Uuid::new_v4().to_string();
    let current_project_id = selected_candidate
        .current_project
        .as_ref()
        .map(|p| p.identity.clone());

    let record = WorkspaceRecord {
        workspace_id,
        canonical_root: selected_candidate.workspace_root.clone(),
        provider_native_key: selected_candidate.workspace_key.clone(),
        capability_bindings: bindings,
        binding_provenance: provenances,
        discovery_fingerprint: selected_candidate.fingerprint.clone(),
        last_successful_validation: current_timestamp(),
        current_project_id,
    };

    // Phase 7: Persist configuration
    if !options.no_configure {
        save_workspace_record(&record)?;
    }

    // Phase 8: Format configuration notification message
    let mut msg_lines = Vec::new();
    msg_lines.push("Configured Staircase workspace:".to_string());
    if ws_provider_name == "core.git" {
        msg_lines.push("  workspace: single Git repository".to_string());
        msg_lines.push(format!(
            "  root: {}",
            selected_candidate.workspace_root.display()
        ));
    } else {
        msg_lines.push(format!("  workspace: {}", ws_provider_name));
        if let Some(ref proj) = record.current_project_id {
            msg_lines.push(format!("  project: {}", proj));
        }
        if let Some(rev_b) = record.capability_bindings.get(&Capability::Review) {
            msg_lines.push(format!("  review: {}", rev_b.provider));
        }
        if let Some(ver_b) = record.capability_bindings.get(&Capability::Verification) {
            msg_lines.push(format!("  verification: {}", ver_b.provider));
        }
    }

    let message = if options.is_porcelain_or_json {
        None
    } else {
        Some(msg_lines.join("\n"))
    };

    Ok(BootstrapResult {
        record,
        newly_configured: true,
        message,
        warning: None,
    })
}

fn select_workspace_candidate<'a>(
    candidates: &'a [WorkspaceCandidate],
    options: &BootstrapOptions,
) -> Result<&'a WorkspaceCandidate> {
    if let Some(ref forced_prov) = options.workspace_provider {
        if let Some(c) = candidates.iter().find(|c| &c.provider == forced_prov) {
            return Ok(c);
        } else {
            return Err(StaircaseError::Other(format!(
                "Requested workspace provider '{}' was not found",
                forced_prov
            )));
        }
    }

    let high_conf_non_core: Vec<&WorkspaceCandidate> = candidates
        .iter()
        .filter(|c| c.provider != "core.git" && (c.confidence == "high" || c.claim == "authoritative"))
        .collect();

    if high_conf_non_core.len() == 1 {
        return Ok(high_conf_non_core[0]);
    } else if high_conf_non_core.len() > 1 {
        return Err(StaircaseError::Ambiguous(format!(
            "Ambiguous workspace providers detected: {}. Please specify --workspace-provider.",
            high_conf_non_core
                .iter()
                .map(|c| c.provider.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    if let Some(fallback) = candidates.iter().find(|c| c.provider == "core.git") {
        Ok(fallback)
    } else {
        Err(StaircaseError::Other(
            "No valid workspace candidate found".to_string(),
        ))
    }
}

fn make_default_core_git_record(workdir: &PathBuf) -> WorkspaceRecord {
    let mut bindings = HashMap::new();
    let mut provenances = HashMap::new();

    bindings.insert(
        Capability::Workspace,
        CapabilityBinding {
            provider: "core.git".to_string(),
            provenance: BindingProvenance::Default,
            evidence: Some("built-in single-Git workspace".to_string()),
        },
    );
    bindings.insert(
        Capability::ProjectMapping,
        CapabilityBinding {
            provider: "core.git".to_string(),
            provenance: BindingProvenance::Default,
            evidence: None,
        },
    );
    bindings.insert(
        Capability::IntegrationContext,
        CapabilityBinding {
            provider: "core.git".to_string(),
            provenance: BindingProvenance::Default,
            evidence: None,
        },
    );
    bindings.insert(
        Capability::Transport,
        CapabilityBinding {
            provider: "git".to_string(),
            provenance: BindingProvenance::Default,
            evidence: None,
        },
    );

    provenances.insert(Capability::Workspace, BindingProvenance::Default);

    WorkspaceRecord {
        workspace_id: uuid::Uuid::new_v4().to_string(),
        canonical_root: workdir.clone(),
        provider_native_key: Some(format!("core.git:{}", workdir.display())),
        capability_bindings: bindings,
        binding_provenance: provenances,
        discovery_fingerprint: HashMap::from([("provider".to_string(), "core.git".to_string())]),
        last_successful_validation: current_timestamp(),
        current_project_id: workdir
            .file_name()
            .map(|s| s.to_string_lossy().to_string()),
    }
}

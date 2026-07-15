use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::workspace::model::{
    BindingProvenance, Capability, CapabilityBinding, CapabilityReadiness, WorkspaceCandidate,
    WorkspaceRecord,
};
use crate::workspace::provider::{
    discover_installed_providers, expand_profile, get_core_git_candidate,
    invoke_provider_probe_workspace,
};
use crate::workspace::repo_provider::probe_repo_workspace;
use crate::workspace::storage::{
    current_timestamp, find_workspace_record_for_path, list_workspace_records,
    load_workspace_record_by_id, save_workspace_record, save_workspace_record_cas,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    let workdir = establish_git_context(repo);

    if options.no_bootstrap {
        let record = find_workspace_record_for_path(&workdir)?
            .unwrap_or_else(|| make_default_core_git_record(&workdir));
        return Ok(BootstrapResult {
            record,
            newly_configured: false,
            message: None,
            warning: None,
        });
    }

    if let Some(mut record) = locate_existing_configuration(repo, &workdir, options)? {
        let expected_generation = record.generation;
        record = reconfigure_existing_record(repo, record, options)?;

        if !options.no_configure {
            save_workspace_record_cas(&record, Some(expected_generation))?;
            record.generation = expected_generation.saturating_add(1);
        }

        return Ok(BootstrapResult {
            record,
            newly_configured: false,
            message: None,
            warning: None,
        });
    }

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

    let candidates = probe_workspace_candidates(repo)?;
    let selected_candidate = select_workspace_candidate(&candidates, options)?;
    let record = initialize_new_record(repo, selected_candidate, options)?;

    if !options.no_configure {
        save_workspace_record(&record)?;
    }

    let message = format_bootstrap_message(&record, options);

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
        .filter(|c| {
            c.provider != "core.git" && (c.confidence == "high" || c.claim == "authoritative")
        })
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
    provenances.insert(Capability::ProjectMapping, BindingProvenance::Default);
    provenances.insert(Capability::IntegrationContext, BindingProvenance::Default);
    provenances.insert(Capability::Transport, BindingProvenance::Default);

    WorkspaceRecord {
        workspace_id: uuid::Uuid::new_v4().to_string(),
        canonical_root: workdir.clone(),
        provider_native_key: Some(format!("core.git:{}", workdir.display())),
        capability_bindings: bindings,
        binding_provenance: provenances,
        capability_readiness: HashMap::from([
            (Capability::Workspace, CapabilityReadiness::Ready),
            (Capability::ProjectMapping, CapabilityReadiness::Ready),
            (Capability::IntegrationContext, CapabilityReadiness::Ready),
            (Capability::Transport, CapabilityReadiness::Ready),
        ]),
        discovery_fingerprint: HashMap::from([("provider".to_string(), "core.git".to_string())]),
        last_successful_validation: current_timestamp(),
        current_project_id: workdir.file_name().map(|s| s.to_string_lossy().to_string()),
        generation: 0,
        extensions: HashMap::new(),
    }
}

fn review_capabilities(provider: &str) -> Vec<Capability> {
    let mut capabilities = vec![
        Capability::Review,
        Capability::ReviewIdentity,
        Capability::Verification,
        Capability::ReviewTransport,
        Capability::Landing,
    ];
    if provider == "github" {
        capabilities.insert(0, Capability::RepositoryRouting);
    }
    capabilities
}

fn bind_capability(
    record: &mut WorkspaceRecord,
    capability: Capability,
    provider: &str,
    provenance: BindingProvenance,
    evidence: Option<String>,
) {
    record.capability_bindings.insert(
        capability,
        CapabilityBinding {
            provider: provider.into(),
            provenance: provenance.clone(),
            evidence,
        },
    );
    record.binding_provenance.insert(capability, provenance);
    record
        .capability_readiness
        .insert(capability, CapabilityReadiness::Ready);
}

fn bind_review_capabilities(
    record: &mut WorkspaceRecord,
    provider: &str,
    provenance: BindingProvenance,
    evidence: Option<String>,
) {
    for capability in review_capabilities(provider) {
        bind_capability(
            record,
            capability,
            provider,
            provenance.clone(),
            evidence.clone(),
        );
    }
}

fn revalidate_record(repo: &GitRepo, record: &mut WorkspaceRecord) -> Result<()> {
    let workspace_provider = record
        .capability_bindings
        .get(&Capability::Workspace)
        .map(|binding| binding.provider.clone())
        .unwrap_or_else(|| "core.git".into());
    if workspace_provider == "repo" {
        match crate::workspace::repo_provider::observe_repo_workspace(repo)? {
            Some(observation) => {
                record.canonical_root = observation.candidate.workspace_root.clone();
                record.current_project_id = Some(observation.mapping.project_name);
                record.discovery_fingerprint = observation.candidate.fingerprint;
                for capability in [
                    Capability::Workspace,
                    Capability::ProjectMapping,
                    Capability::IntegrationContext,
                    Capability::WorkspaceHints,
                ] {
                    record
                        .capability_readiness
                        .insert(capability, CapabilityReadiness::Ready);
                }
            }
            None => {
                for capability in [
                    Capability::Workspace,
                    Capability::ProjectMapping,
                    Capability::IntegrationContext,
                    Capability::WorkspaceHints,
                ] {
                    record
                        .capability_readiness
                        .insert(capability, CapabilityReadiness::Stale);
                }
            }
        }
    }

    let existing_review = record
        .capability_bindings
        .get(&Capability::Review)
        .map(|binding| (binding.provider.clone(), binding.provenance.clone()));
    let provider = existing_review
        .as_ref()
        .map(|(provider, _)| provider.clone())
        .or_else(|| {
            crate::workspace::gerrit_provider::probe_gerrit_route(repo, Some(record))
                .ok()
                .flatten()
                .map(|_| "gerrit".to_string())
        })
        .or_else(|| {
            crate::workspace::github_provider::probe_github_route(repo, Some(record))
                .ok()
                .flatten()
                .map(|_| "github".to_string())
        });
    if let Some(provider) = provider {
        let ready = match provider.as_str() {
            "gerrit" => {
                crate::workspace::gerrit_provider::probe_gerrit_route(repo, Some(record))?.is_some()
            }
            "github" => {
                crate::workspace::github_provider::probe_github_route(repo, Some(record))?.is_some()
            }
            _ => false,
        };
        let provenance = existing_review
            .map(|(_, provenance)| provenance)
            .unwrap_or(BindingProvenance::AutoDiscovered);
        if !record.capability_bindings.contains_key(&Capability::Review) {
            bind_review_capabilities(
                record,
                &provider,
                provenance,
                Some(format!("{} route discovered", provider)),
            );
        }
        for capability in review_capabilities(&provider) {
            record.capability_readiness.insert(
                capability,
                if ready {
                    CapabilityReadiness::Ready
                } else {
                    CapabilityReadiness::RouteIncomplete
                },
            );
        }
    }
    record.last_successful_validation = current_timestamp();
    Ok(())
}

pub(crate) fn establish_git_context(repo: &GitRepo) -> PathBuf {
    repo.workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone())
}

pub(crate) fn locate_existing_configuration(
    repo: &GitRepo,
    workdir: &Path,
    options: &BootstrapOptions,
) -> Result<Option<WorkspaceRecord>> {
    if let Some(ref ws_id) = options.workspace_id {
        if let Some(record) = load_workspace_record_by_id(ws_id)? {
            return Ok(Some(record));
        }
    }

    let search_dir = if let Ok(Some(shadow_entry)) =
        crate::monorepo::find_shadow_worktree_for_path(workdir)
    {
        shadow_entry.primary_root.clone()
    } else {
        workdir.to_path_buf()
    };

    let mut existing = find_workspace_record_for_path(&search_dir)?;
    if existing.is_none() {
        if let Some(candidate) = probe_repo_workspace(repo)? {
            existing = list_workspace_records()?.into_iter().find(|record| {
                record.provider_native_key.is_some()
                    && record.provider_native_key == candidate.workspace_key
            });
        }
    }
    Ok(existing)
}

pub(crate) fn reconfigure_existing_record(
    repo: &GitRepo,
    mut record: WorkspaceRecord,
    options: &BootstrapOptions,
) -> Result<WorkspaceRecord> {
    if options.workspace_mode.as_deref() == Some("single-git") {
        bind_capability(
            &mut record,
            Capability::Workspace,
            "core.git",
            BindingProvenance::Explicit,
            Some("explicit --workspace-mode=single-git".into()),
        );
    }
    if let Some(profile) = &options.provider_profile {
        for (capability, provider) in expand_profile(profile) {
            if record
                .capability_bindings
                .get(&capability)
                .is_some_and(|binding| binding.provenance == BindingProvenance::Explicit)
            {
                continue;
            }
            bind_capability(
                &mut record,
                capability,
                &provider,
                BindingProvenance::Profile,
                Some(format!("profile {}", profile)),
            );
        }
    }
    if let Some(provider) = &options.workspace_provider {
        bind_capability(
            &mut record,
            Capability::Workspace,
            provider,
            BindingProvenance::Explicit,
            Some("explicit workspace provider".into()),
        );
    }
    if let Some(provider) = &options.review_provider {
        bind_review_capabilities(
            &mut record,
            provider,
            BindingProvenance::Explicit,
            Some("explicit review provider".into()),
        );
    }
    revalidate_record(repo, &mut record)?;
    Ok(record)
}

pub(crate) fn probe_workspace_candidates(repo: &GitRepo) -> Result<Vec<WorkspaceCandidate>> {
    let installed = discover_installed_providers()?;
    let mut candidates = Vec::new();

    for prov in &installed {
        if let Ok(Some(cand)) = invoke_provider_probe_workspace(prov, repo) {
            candidates.push(cand);
        }
    }

    if let Ok(Some(repo_cand)) = probe_repo_workspace(repo) {
        candidates.push(repo_cand);
    }

    let fallback_cand = get_core_git_candidate(repo);
    candidates.push(fallback_cand);
    Ok(candidates)
}

pub(crate) fn initialize_new_record(
    repo: &GitRepo,
    selected_candidate: &WorkspaceCandidate,
    options: &BootstrapOptions,
) -> Result<WorkspaceRecord> {
    let mut bindings = HashMap::new();
    let mut provenances = HashMap::new();

    let ws_provider_name = selected_candidate.provider.clone();

    let workspace_provenance = if options.workspace_provider.is_some() {
        BindingProvenance::Explicit
    } else if ws_provider_name == "core.git" {
        BindingProvenance::Default
    } else {
        BindingProvenance::AutoDiscovered
    };
    bindings.insert(
        Capability::Workspace,
        CapabilityBinding {
            provider: ws_provider_name.clone(),
            provenance: workspace_provenance.clone(),
            evidence: selected_candidate.evidence.first().cloned(),
        },
    );
    provenances.insert(Capability::Workspace, workspace_provenance);

    bindings.insert(
        Capability::ProjectMapping,
        CapabilityBinding {
            provider: ws_provider_name.clone(),
            provenance: BindingProvenance::AutoDiscovered,
            evidence: None,
        },
    );
    provenances.insert(
        Capability::ProjectMapping,
        BindingProvenance::AutoDiscovered,
    );
    bindings.insert(
        Capability::IntegrationContext,
        CapabilityBinding {
            provider: ws_provider_name.clone(),
            provenance: BindingProvenance::AutoDiscovered,
            evidence: None,
        },
    );
    provenances.insert(
        Capability::IntegrationContext,
        BindingProvenance::AutoDiscovered,
    );
    if ws_provider_name == "repo" {
        bindings.insert(
            Capability::WorkspaceHints,
            CapabilityBinding {
                provider: "repo".into(),
                provenance: BindingProvenance::AutoDiscovered,
                evidence: Some("effective repo manifest hints".into()),
            },
        );
        provenances.insert(
            Capability::WorkspaceHints,
            BindingProvenance::AutoDiscovered,
        );
    }
    bindings.insert(
        Capability::Transport,
        CapabilityBinding {
            provider: "git".to_string(),
            provenance: BindingProvenance::Default,
            evidence: None,
        },
    );
    provenances.insert(Capability::Transport, BindingProvenance::Default);

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

    if let Some(ref rev_prov) = options.review_provider {
        for capability in review_capabilities(rev_prov) {
            bindings.insert(
                capability,
                CapabilityBinding {
                    provider: rev_prov.clone(),
                    provenance: BindingProvenance::Explicit,
                    evidence: Some("explicit review provider".into()),
                },
            );
            provenances.insert(capability, BindingProvenance::Explicit);
        }
    }

    let temp_record = WorkspaceRecord {
        workspace_id: uuid::Uuid::new_v4().to_string(),
        canonical_root: selected_candidate.workspace_root.clone(),
        provider_native_key: selected_candidate.workspace_key.clone(),
        capability_bindings: bindings.clone(),
        binding_provenance: provenances.clone(),
        capability_readiness: bindings
            .keys()
            .copied()
            .map(|capability| (capability, CapabilityReadiness::Ready))
            .collect(),
        discovery_fingerprint: selected_candidate.fingerprint.clone(),
        last_successful_validation: current_timestamp(),
        current_project_id: selected_candidate
            .current_project
            .as_ref()
            .map(|p| p.identity.clone()),
        generation: 0,
        extensions: HashMap::new(),
    };

    if !bindings.contains_key(&Capability::Review) {
        if let Ok(Some(_gerrit_route)) =
            crate::workspace::gerrit_provider::probe_gerrit_route(repo, Some(&temp_record))
        {
            bindings.insert(
                Capability::Review,
                CapabilityBinding {
                    provider: "gerrit".to_string(),
                    provenance: BindingProvenance::AutoDiscovered,
                    evidence: Some("Gerrit review route discovered".to_string()),
                },
            );
            provenances.insert(Capability::Review, BindingProvenance::AutoDiscovered);

            bindings.insert(
                Capability::Verification,
                CapabilityBinding {
                    provider: "gerrit".to_string(),
                    provenance: BindingProvenance::AutoDiscovered,
                    evidence: Some("Gerrit verification route discovered".to_string()),
                },
            );
            provenances.insert(Capability::Verification, BindingProvenance::AutoDiscovered);
            for capability in [
                Capability::ReviewIdentity,
                Capability::ReviewTransport,
                Capability::Landing,
            ] {
                bindings.insert(
                    capability,
                    CapabilityBinding {
                        provider: "gerrit".into(),
                        provenance: BindingProvenance::AutoDiscovered,
                        evidence: Some("Gerrit route discovered".into()),
                    },
                );
                provenances.insert(capability, BindingProvenance::AutoDiscovered);
            }
        } else if let Ok(Some(_gh_route)) =
            crate::workspace::github_provider::probe_github_route(repo, Some(&temp_record))
        {
            bindings.insert(
                Capability::Review,
                CapabilityBinding {
                    provider: "github".to_string(),
                    provenance: BindingProvenance::AutoDiscovered,
                    evidence: Some("GitHub review route discovered".to_string()),
                },
            );
            provenances.insert(Capability::Review, BindingProvenance::AutoDiscovered);

            bindings.insert(
                Capability::Verification,
                CapabilityBinding {
                    provider: "github".to_string(),
                    provenance: BindingProvenance::AutoDiscovered,
                    evidence: Some("GitHub verification route discovered".to_string()),
                },
            );
            provenances.insert(Capability::Verification, BindingProvenance::AutoDiscovered);
            for capability in [
                Capability::RepositoryRouting,
                Capability::ReviewIdentity,
                Capability::ReviewTransport,
                Capability::Landing,
            ] {
                bindings.insert(
                    capability,
                    CapabilityBinding {
                        provider: "github".into(),
                        provenance: BindingProvenance::AutoDiscovered,
                        evidence: Some("GitHub repository route discovered".into()),
                    },
                );
                provenances.insert(capability, BindingProvenance::AutoDiscovered);
            }
        }
    }

    let workspace_id = uuid::Uuid::new_v4().to_string();
    let current_project_id = selected_candidate
        .current_project
        .as_ref()
        .map(|p| p.identity.clone());
    let capability_readiness = Capability::ALL
        .into_iter()
        .map(|capability| {
            (
                capability,
                if bindings.contains_key(&capability) {
                    CapabilityReadiness::Ready
                } else {
                    CapabilityReadiness::Unavailable
                },
            )
        })
        .collect();

    Ok(WorkspaceRecord {
        workspace_id,
        canonical_root: selected_candidate.workspace_root.clone(),
        provider_native_key: selected_candidate.workspace_key.clone(),
        capability_bindings: bindings,
        binding_provenance: provenances,
        capability_readiness,
        discovery_fingerprint: selected_candidate.fingerprint.clone(),
        last_successful_validation: current_timestamp(),
        current_project_id,
        generation: 0,
        extensions: HashMap::new(),
    })
}

pub(crate) fn format_bootstrap_message(
    record: &WorkspaceRecord,
    options: &BootstrapOptions,
) -> Option<String> {
    if options.is_porcelain_or_json {
        return None;
    }

    let mut msg_lines = Vec::new();
    msg_lines.push("Configured Staircase workspace:".to_string());

    let ws_provider_name = record
        .capability_bindings
        .get(&Capability::Workspace)
        .map(|b| b.provider.as_str())
        .unwrap_or("unknown");

    if ws_provider_name == "core.git" {
        msg_lines.push("  workspace: single Git repository".to_string());
        msg_lines.push(format!("  root: {}", record.canonical_root.display()));
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
    Some(msg_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, TempDir, GitRepo) {
        let tmp = TempDir::new().unwrap();
        let storage_tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::process::Command::new("git")
            .current_dir(&path)
            .args(&["init", "-b", "main"])
            .output()
            .unwrap();
        let repo = GitRepo::new(path);
        unsafe {
            std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_tmp.path());
        }
        (tmp, storage_tmp, repo)
    }

    #[test]
    fn test_establish_git_context() {
        let (_tmp, _storage, repo) = setup_test_repo();
        let context = establish_git_context(&repo);
        assert!(context.exists());
        assert!(context.is_absolute());
    }

    #[test]
    fn test_locate_existing_configuration_none() {
        let (_tmp, _storage, repo) = setup_test_repo();
        let workdir = establish_git_context(&repo);
        let options = BootstrapOptions::default();
        let existing = locate_existing_configuration(&repo, &workdir, &options).unwrap();
        assert!(
            existing.is_none(),
            "Expected no existing configuration for a new repo, but found one."
        );
    }

    #[test]
    fn test_format_bootstrap_message_core_git() {
        let workdir = PathBuf::from("/tmp/repo");
        let record = make_default_core_git_record(&workdir);
        let options = BootstrapOptions::default();
        let message = format_bootstrap_message(&record, &options).unwrap();
        assert!(message.contains("single Git repository"));
        assert!(message.contains("/tmp/repo"));
    }
}

use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::workspace::model::{
    Capability, ProbeDescriptor, ProviderDescriptor, WorkspaceCandidate,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub fn get_core_git_descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        protocol_version: 1,
        name: "core.git".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            Capability::Workspace,
            Capability::ProjectMapping,
            Capability::IntegrationContext,
            Capability::WorkspaceHints,
            Capability::RepositoryRouting,
            Capability::Transport,
        ],
        probe: ProbeDescriptor {
            passive: true,
            network: false,
            authenticates: false,
            mutates_workspace: false,
            executes_repository_hooks: false,
        },
    }
}

pub fn get_core_git_candidate(repo: &GitRepo) -> WorkspaceCandidate {
    let root = repo
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone());
    WorkspaceCandidate {
        provider: "core.git".to_string(),
        workspace_root: root.clone(),
        workspace_key: Some(format!("core.git:{}", root.display())),
        current_project: Some(crate::workspace::model::ProjectInfo {
            path: PathBuf::from("."),
            identity: root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "git-repo".to_string()),
        }),
        claim: "fallback".to_string(),
        confidence: "low".to_string(),
        evidence: vec!["built-in single-Git repository fallback".to_string()],
        fingerprint: HashMap::from([
            ("provider".to_string(), "core.git".to_string()),
            ("root".to_string(), root.display().to_string()),
        ]),
    }
}

pub fn get_provider_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(override_dir) = std::env::var("GIT_STAIRCASE_PROVIDER_DIR") {
        dirs.push(PathBuf::from(override_dir));
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        dirs.push(PathBuf::from(xdg).join("git-staircase").join("providers"));
    }
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        dirs.push(
            home_path
                .join(".config")
                .join("git-staircase")
                .join("providers"),
        );
        dirs.push(
            home_path
                .join(".local")
                .join("share")
                .join("git-staircase")
                .join("providers"),
        );
    }
    dirs
}

pub struct InstalledProvider {
    pub descriptor: ProviderDescriptor,
    pub executable_path: PathBuf,
}

pub fn discover_installed_providers() -> Result<Vec<InstalledProvider>> {
    let mut providers = Vec::new();
    let dirs = get_provider_directories();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    #[cfg(unix)]
                    let is_exec = {
                        use std::os::unix::fs::PermissionsExt;
                        entry
                            .metadata()
                            .map(|m| m.permissions().mode() & 0o111 != 0)
                            .unwrap_or(false)
                    };
                    #[cfg(not(unix))]
                    let is_exec = true;

                    if is_exec {
                        if let Ok(descriptor) = query_provider_descriptor(&path) {
                            if descriptor.probe.passive
                                && !descriptor.probe.network
                                && !descriptor.probe.mutates_workspace
                            {
                                providers.push(InstalledProvider {
                                    descriptor,
                                    executable_path: path,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(providers)
}

fn wait_with_timeout(mut child: Child, timeout: Duration) -> Result<Output> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|e| StaircaseError::Other(e.to_string()));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(StaircaseError::Other(format!(
                        "Process timed out after {:?}",
                        timeout
                    )));
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(StaircaseError::Other(e.to_string())),
        }
    }
}

pub fn query_provider_descriptor(exe: &Path) -> Result<ProviderDescriptor> {
    let child = Command::new(exe)
        .arg("describe")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| StaircaseError::Other(format!("Failed to run provider describe: {}", e)))?;

    let output = wait_with_timeout(child, Duration::from_secs(1))?;

    if !output.status.success() {
        return Err(StaircaseError::Other(format!(
            "Provider describe failed for {}",
            exe.display()
        )));
    }

    let descriptor: ProviderDescriptor = serde_json::from_slice(&output.stdout).map_err(|e| {
        StaircaseError::Other(format!(
            "Invalid provider descriptor from {}: {}",
            exe.display(),
            e
        ))
    })?;

    Ok(descriptor)
}

pub fn invoke_provider_probe_workspace(
    provider: &InstalledProvider,
    repo: &GitRepo,
) -> Result<Option<WorkspaceCandidate>> {
    let input_json = serde_json::json!({
        "protocol_version": 1,
        "operation": "probe-workspace",
        "cwd": repo.workdir.to_string_lossy(),
        "git_common_dir": repo.workdir.join(".git").to_string_lossy(),
        "network_allowed": false
    });

    let mut child = Command::new(&provider.executable_path)
        .arg("probe-workspace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| StaircaseError::Other(format!("Failed to spawn provider: {}", e)))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(input_json.to_string().as_bytes());
    }

    let output = wait_with_timeout(child, Duration::from_secs(5))?;

    if !output.status.success() {
        return Ok(None);
    }

    let candidate: Option<WorkspaceCandidate> = serde_json::from_slice(&output.stdout)
        .map_err(|e| StaircaseError::Other(format!("Invalid candidate from provider: {}", e)))?;

    Ok(candidate)
}

pub fn expand_profile(profile: &str) -> HashMap<Capability, String> {
    let mut bindings = HashMap::new();
    match profile {
        "repo+gerrit" => {
            bindings.insert(Capability::Workspace, "repo".to_string());
            bindings.insert(Capability::ProjectMapping, "repo".to_string());
            bindings.insert(Capability::IntegrationContext, "repo".to_string());
            bindings.insert(Capability::WorkspaceHints, "repo".to_string());
            bindings.insert(Capability::Review, "gerrit".to_string());
            bindings.insert(Capability::ReviewIdentity, "gerrit".to_string());
            bindings.insert(Capability::Verification, "gerrit".to_string());
            bindings.insert(Capability::ReviewTransport, "gerrit".to_string());
            bindings.insert(Capability::Landing, "gerrit".to_string());
        }
        "single-git" => {
            bindings.insert(Capability::Workspace, "core.git".to_string());
            bindings.insert(Capability::ProjectMapping, "core.git".to_string());
            bindings.insert(Capability::IntegrationContext, "core.git".to_string());
            bindings.insert(Capability::WorkspaceHints, "core.git".to_string());
            bindings.insert(Capability::RepositoryRouting, "core.git".to_string());
            bindings.insert(Capability::Transport, "git".to_string());
        }
        _ => {}
    }
    bindings
}

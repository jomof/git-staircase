use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Workspace,
    ProjectMapping,
    IntegrationContext,
    Review,
    Verification,
    Transport,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::Workspace => write!(f, "workspace"),
            Capability::ProjectMapping => write!(f, "project-mapping"),
            Capability::IntegrationContext => write!(f, "integration-context"),
            Capability::Review => write!(f, "review"),
            Capability::Verification => write!(f, "verification"),
            Capability::Transport => write!(f, "transport"),
        }
    }
}

impl std::str::FromStr for Capability {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "workspace" => Ok(Capability::Workspace),
            "project-mapping" => Ok(Capability::ProjectMapping),
            "integration-context" => Ok(Capability::IntegrationContext),
            "review" => Ok(Capability::Review),
            "verification" => Ok(Capability::Verification),
            "transport" => Ok(Capability::Transport),
            _ => Err(format!("Unknown capability: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingProvenance {
    Explicit,
    Profile,
    AutoDiscovered,
    Inherited,
    Default,
}

impl fmt::Display for BindingProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindingProvenance::Explicit => write!(f, "explicit"),
            BindingProvenance::Profile => write!(f, "profile"),
            BindingProvenance::AutoDiscovered => write!(f, "auto-discovered"),
            BindingProvenance::Inherited => write!(f, "inherited"),
            BindingProvenance::Default => write!(f, "default"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityBinding {
    pub provider: String,
    pub provenance: BindingProvenance,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub workspace_id: String,
    pub canonical_root: PathBuf,
    pub provider_native_key: Option<String>,
    pub capability_bindings: HashMap<Capability, CapabilityBinding>,
    pub binding_provenance: HashMap<Capability, BindingProvenance>,
    pub discovery_fingerprint: HashMap<String, String>,
    pub last_successful_validation: u64,
    pub current_project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeDescriptor {
    pub passive: bool,
    pub network: bool,
    pub mutates_workspace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub protocol_version: u32,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub probe: ProbeDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: PathBuf,
    pub identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceCandidate {
    pub provider: String,
    pub workspace_root: PathBuf,
    pub workspace_key: Option<String>,
    pub current_project: Option<ProjectInfo>,
    pub claim: String,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub fingerprint: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProbeOutput {
    pub applicable: bool,
    pub confidence: String,
    pub facts: HashMap<String, String>,
    pub hints: HashMap<String, String>,
    pub requirements: HashMap<String, bool>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationContextCandidate {
    pub anchor_oid: Option<String>,
    pub symbolic_target: Option<String>,
    pub mode: String,
    pub provenance: String,
}

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
    WorkspaceHints,
    RepositoryRouting,
    Review,
    ReviewIdentity,
    Verification,
    ReviewTransport,
    Transport,
    Landing,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::Workspace => write!(f, "workspace"),
            Capability::ProjectMapping => write!(f, "project-mapping"),
            Capability::IntegrationContext => write!(f, "integration-context"),
            Capability::WorkspaceHints => write!(f, "workspace-hints"),
            Capability::RepositoryRouting => write!(f, "repository-routing"),
            Capability::Review => write!(f, "review"),
            Capability::ReviewIdentity => write!(f, "review-identity"),
            Capability::Verification => write!(f, "verification"),
            Capability::ReviewTransport => write!(f, "review-transport"),
            Capability::Transport => write!(f, "transport"),
            Capability::Landing => write!(f, "landing"),
        }
    }
}

impl Capability {
    pub const ALL: [Capability; 11] = [
        Capability::Workspace,
        Capability::ProjectMapping,
        Capability::IntegrationContext,
        Capability::WorkspaceHints,
        Capability::RepositoryRouting,
        Capability::Review,
        Capability::ReviewIdentity,
        Capability::Verification,
        Capability::ReviewTransport,
        Capability::Transport,
        Capability::Landing,
    ];
}

impl std::str::FromStr for Capability {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "workspace" => Ok(Capability::Workspace),
            "project-mapping" => Ok(Capability::ProjectMapping),
            "integration-context" => Ok(Capability::IntegrationContext),
            "workspace-hints" => Ok(Capability::WorkspaceHints),
            "repository-routing" => Ok(Capability::RepositoryRouting),
            "review" => Ok(Capability::Review),
            "review-identity" => Ok(Capability::ReviewIdentity),
            "verification" => Ok(Capability::Verification),
            "review-transport" => Ok(Capability::ReviewTransport),
            "transport" => Ok(Capability::Transport),
            "landing" => Ok(Capability::Landing),
            _ => Err(format!("Unknown capability: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityReadiness {
    Ready,
    Degraded,
    RouteIncomplete,
    Unavailable,
    Stale,
}

impl Default for CapabilityReadiness {
    fn default() -> Self {
        Self::Ready
    }
}

impl fmt::Display for CapabilityReadiness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready => write!(f, "ready"),
            Self::Degraded => write!(f, "degraded"),
            Self::RouteIncomplete => write!(f, "route-incomplete"),
            Self::Unavailable => write!(f, "unavailable"),
            Self::Stale => write!(f, "stale"),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityBinding {
    pub provider: String,
    pub provenance: BindingProvenance,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub workspace_id: String,
    pub canonical_root: PathBuf,
    pub provider_native_key: Option<String>,
    pub capability_bindings: HashMap<Capability, CapabilityBinding>,
    pub binding_provenance: HashMap<Capability, BindingProvenance>,
    #[serde(default)]
    pub capability_readiness: HashMap<Capability, CapabilityReadiness>,
    pub discovery_fingerprint: HashMap<String, String>,
    pub last_successful_validation: u64,
    pub current_project_id: Option<String>,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeDescriptor {
    pub passive: bool,
    pub network: bool,
    #[serde(default)]
    pub authenticates: bool,
    pub mutates_workspace: bool,
    #[serde(default)]
    pub executes_repository_hooks: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceAuthority {
    Authoritative,
    Strong,
    Advisory,
    Observed,
    Ineligible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedEvidence {
    pub kind: String,
    pub locator: Option<String>,
    pub resolved_oid: Option<String>,
    pub source: String,
    pub exact: bool,
    pub moving: bool,
    pub authority: EvidenceAuthority,
    pub provenance: Vec<String>,
    pub eligible: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceHint {
    pub kind: String,
    pub provider_hint: Option<String>,
    pub value: String,
    pub source: String,
    pub scope: HashMap<String, String>,
    pub confidence: String,
    pub normalized: bool,
    pub freshness_fingerprint: HashMap<String, String>,
}

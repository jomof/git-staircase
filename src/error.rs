use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExitClass {
    Usage,
    Selection,
    ConcurrentState,
    OperationConflict,
    Provider,
    Policy,
    Integrity,
}

impl ExitClass {
    pub const fn status(self) -> i32 {
        match self {
            Self::Usage => 1,
            Self::Selection => 2,
            Self::ConcurrentState => 3,
            Self::OperationConflict => 4,
            Self::Provider => 5,
            Self::Policy => 6,
            Self::Integrity => 7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AmbiguityCandidate {
    pub kind: String,
    pub selector: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_oid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_context: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<String>,
}

#[derive(Error, Debug)]
pub enum StaircaseError {
    #[error("Git command failed: {command}\nStdout: {stdout}\nStderr: {stderr}")]
    GitCommandFailed {
        command: String,
        stdout: String,
        stderr: String,
    },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Staircase not found: {0}")]
    NotFound(String),
    #[error("Ambiguous staircase: {0}")]
    Ambiguous(String),
    #[error("selector '{selector}' is ambiguous")]
    SelectorAmbiguous {
        selector: String,
        candidates: Vec<AmbiguityCandidate>,
    },
    #[error("record changed concurrently for {reference}: expected {expected}, actual {actual}")]
    ConcurrentRecordUpdate {
        reference: String,
        expected: String,
        actual: String,
    },
    #[error("Staircase operation {operation_id} ({kind}) is already in progress")]
    OperationInProgress { operation_id: String, kind: String },
    #[error("Staircase operation {operation_id} ({kind}) paused for conflict resolution")]
    OperationPaused { operation_id: String, kind: String },
    #[error("No Staircase operation is in progress")]
    NoActiveOperation,
    #[error("ref collision at {reference}: expected {expected}, actual {actual}")]
    RefCollision {
        reference: String,
        expected: String,
        actual: String,
    },
    #[error("unsupported topology for {operation}: {reason}")]
    UnsupportedTopology { operation: String, reason: String },
    #[error("external Git operation '{operation}' is owned by {owner}")]
    ExternalOperation { operation: String, owner: String },
    #[error("Invalid staircase structure: {0}")]
    InvalidStructure(String),
    #[error("adoption required for {operation}, but inhibited by --no-adopt: {reason}")]
    AdoptionInhibited { operation: String, reason: String },
    #[error("Other error: {0}")]
    Other(String),
}

impl StaircaseError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "selector-not-found",
            Self::Ambiguous(_) | Self::SelectorAmbiguous { .. } => "selector-ambiguous",
            Self::ConcurrentRecordUpdate { .. } => "concurrent-record-update",
            Self::OperationInProgress { .. } => "operation-in-progress",
            Self::OperationPaused { .. } => "operation-paused",
            Self::NoActiveOperation => "no-active-operation",
            Self::RefCollision { .. } => "ref-collision",
            Self::UnsupportedTopology { .. } => "unsupported-topology",
            Self::ExternalOperation { .. } => "external-operation-in-progress",
            Self::InvalidStructure(_) => "invalid-cut-chain",
            Self::AdoptionInhibited { .. } => "adoption-required",
            Self::GitCommandFailed { .. } => "git-command-failed",
            Self::Io(_) => "io-error",
            Self::Serialization(_) => "serialization-error",
            Self::Other(_) => "validation-failed",
        }
    }

    pub const fn exit_class(&self) -> ExitClass {
        match self {
            Self::NotFound(_) | Self::Ambiguous(_) | Self::SelectorAmbiguous { .. } => {
                ExitClass::Selection
            }
            Self::ConcurrentRecordUpdate { .. } => ExitClass::ConcurrentState,
            Self::OperationInProgress { .. }
            | Self::OperationPaused { .. }
            | Self::NoActiveOperation
            | Self::ExternalOperation { .. } => ExitClass::OperationConflict,
            Self::RefCollision { .. } => ExitClass::ConcurrentState,
            Self::UnsupportedTopology { .. } | Self::AdoptionInhibited { .. } => ExitClass::Policy,
            Self::InvalidStructure(_) | Self::Other(_) => ExitClass::Usage,
            Self::GitCommandFailed { .. } | Self::Io(_) | Self::Serialization(_) => {
                ExitClass::Integrity
            }
        }
    }

    pub fn details(&self) -> serde_json::Value {
        match self {
            Self::SelectorAmbiguous {
                selector,
                candidates,
            } => serde_json::json!({
                "selector": selector,
                "candidates": candidates,
            }),
            Self::ConcurrentRecordUpdate {
                reference,
                expected,
                actual,
            } => serde_json::json!({
                "ref": reference,
                "expected_record_oid": expected,
                "actual_record_oid": actual,
            }),
            Self::OperationInProgress { operation_id, kind } => serde_json::json!({
                "operation_id": operation_id,
                "operation_kind": kind,
                "continue": ["git", "staircase", "continue"],
                "abort": ["git", "staircase", "abort"],
            }),
            Self::OperationPaused { operation_id, kind } => serde_json::json!({
                "operation_id": operation_id,
                "operation_kind": kind,
                "continue": ["git", "staircase", "continue"],
                "abort": ["git", "staircase", "abort"],
            }),
            Self::RefCollision {
                reference,
                expected,
                actual,
            } => serde_json::json!({
                "ref": reference,
                "expected_oid": expected,
                "actual_oid": actual,
            }),
            Self::UnsupportedTopology { operation, reason } => serde_json::json!({
                "operation": operation,
                "reason": reason,
            }),
            Self::ExternalOperation { operation, owner } => serde_json::json!({
                "operation": operation,
                "owner": owner,
            }),
            Self::AdoptionInhibited { operation, reason } => serde_json::json!({
                "adopted": false,
                "adoption_required": true,
                "adoption_reason": reason,
                "operation": operation,
            }),
            Self::GitCommandFailed {
                command,
                stdout,
                stderr,
            } => serde_json::json!({
                "command": command,
                "stdout": stdout,
                "stderr": stderr,
            }),
            _ => serde_json::Value::Null,
        }
    }
}

pub type Result<T> = std::result::Result<T, StaircaseError>;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleState {
    Active,
    Archived,
}

impl Default for LifecycleState {
    fn default() -> Self {
        LifecycleState::Active
    }
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleState::Active => write!(f, "active"),
            LifecycleState::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LifecycleEvent {
    pub event_id: String,
    pub kind: String,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_oid_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_oid_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub details: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseLifecycle {
    pub state: LifecycleState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_reason: Option<String>,
    #[serde(default = "default_true")]
    pub name_reserved: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<LifecycleEvent>,
}

fn default_true() -> bool {
    true
}

impl Default for StaircaseLifecycle {
    fn default() -> Self {
        Self {
            state: LifecycleState::Active,
            archive_reason: None,
            name_reserved: true,
            events: Vec::new(),
        }
    }
}

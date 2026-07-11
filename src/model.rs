use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub cut: String,            // Commit OID
    pub branch: Option<String>, // Optional local branch name (ref name without refs/heads/)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VerificationPolicy {
    pub build_command: Option<String>,
    pub test_command: Option<String>,
    pub verify_each_prefix: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseMetadata {
    pub id: String,     // UUID
    pub name: String,   // Nominal name
    pub target: String, // Integration boundary (e.g., "refs/remotes/origin/main" or "main")
    pub steps: Vec<Step>,
    pub verification_policy: Option<VerificationPolicy>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FamilyStep {
    pub name: String,
    pub cut: String,
    pub branch: Option<String>,
    pub children: Vec<String>, // Names of child steps
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseFamily {
    pub id: String,
    pub name: String,
    pub target: String,
    pub steps: HashMap<String, FamilyStep>,
    pub roots: Vec<String>, // Names of root steps
    pub verification_policy: Option<VerificationPolicy>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Discovery {
    Linear(StaircaseMetadata),
    Ambiguous(StaircaseFamily),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "management", rename_all = "kebab-case")]
pub enum ResolvedStaircase {
    Managed(StaircaseMetadata),
    Implicit(StaircaseMetadata),
}

impl ResolvedStaircase {
    pub fn metadata(&self) -> &StaircaseMetadata {
        match self {
            ResolvedStaircase::Managed(s) => s,
            ResolvedStaircase::Implicit(s) => s,
        }
    }

    pub fn is_managed(&self) -> bool {
        matches!(self, ResolvedStaircase::Managed(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    pub refname: String, // e.g. "refs/heads/feature/auth-core"
    pub oid: String,
    pub upstream: Option<String>, // e.g. "refs/remotes/origin/main" or "refs/heads/feature/auth-core"
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StepStatus {
    pub name: String,
    pub expected_cut: String,
    pub actual_oid: Option<String>,
    pub is_stale: bool,
    pub is_modified: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StaircaseStatus {
    pub metadata: StaircaseMetadata,
    pub steps: Vec<StepStatus>,
    pub is_clean: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(ValueEnum)]
pub enum IdentityKind {
    Lineage,
    Revision,
    Body,
    Decomposition,
    Outcome,
    PatchSeries,
    Nominal,
    Review,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
    pub step_name: String,
    pub cut: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait ToPorcelain {
    fn to_porcelain(&self) -> String;
}

impl ToPorcelain for StaircaseMetadata {
    fn to_porcelain(&self) -> String {
        format!("{} {}", self.name, self.id)
    }
}

impl ToPorcelain for ResolvedStaircase {
    fn to_porcelain(&self) -> String {
        let m = self.metadata();
        format!(
            "{}\t{}\t{}\t{}",
            m.name,
            m.id,
            if self.is_managed() {
                "managed"
            } else {
                "implicit"
            },
            m.steps.len()
        )
    }
}

impl ToPorcelain for StaircaseStatus {
    fn to_porcelain(&self) -> String {
        let mut out = format!(
            "{}\t{}\t{}\n",
            self.metadata.name,
            self.metadata.id,
            if self.is_clean { "clean" } else { "modified" }
        );
        for step in &self.steps {
            out.push_str(&format!(
                "step\t{}\t{}\t{}\t{}\n",
                step.name,
                step.actual_oid.as_deref().unwrap_or("none"),
                if step.is_modified {
                    "modified"
                } else {
                    "clean"
                },
                if step.is_stale { "stale" } else { "up-to-date" }
            ));
        }
        out
    }
}

impl ToPorcelain for Discovery {
    fn to_porcelain(&self) -> String {
        match self {
            Discovery::Linear(s) => format!("linear\t{}\t{}", s.name, s.steps.len()),
            Discovery::Ambiguous(f) => format!("ambiguous\t{}\t{}", f.name, f.steps.len()),
        }
    }
}

impl ToPorcelain for VerificationResult {
    fn to_porcelain(&self) -> String {
        format!(
            "{}\t{}\t{}",
            self.step_name,
            if self.success { "pass" } else { "fail" },
            self.cut
        )
    }
}

impl<T: ToPorcelain> ToPorcelain for Vec<T> {
    fn to_porcelain(&self) -> String {
        self.iter()
            .map(|x| x.to_porcelain())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

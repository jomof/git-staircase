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

pub trait ToHuman {
    fn to_human(&self) -> String;
}

impl ToHuman for StaircaseMetadata {
    fn to_human(&self) -> String {
        let mut out = format!("  Name: {}\n", self.name);
        out.push_str(&format!("  ID: {}\n", self.id));
        out.push_str(&format!("  Target: {}\n", self.target));
        if let Some(ref policy) = self.verification_policy {
            out.push_str("  Verification Policy:\n");
            if let Some(ref cmd) = policy.build_command {
                out.push_str(&format!("    Build: {}\n", cmd));
            }
            if let Some(ref cmd) = policy.test_command {
                out.push_str(&format!("    Test:  {}\n", cmd));
            }
            out.push_str(&format!(
                "    Verify each prefix: {}\n",
                policy.verify_each_prefix
            ));
        }
        out.push_str("  Steps:\n");
        for (i, step) in self.steps.iter().enumerate() {
            out.push_str(&format!("    Step {}:\n", i + 1));
            out.push_str(&format!("      Name: {}\n", step.name));
            out.push_str(&format!("      Cut: {}\n", step.cut));
            if let Some(ref b) = step.branch {
                out.push_str(&format!("      Branch: {}\n", b));
            }
        }
        out
    }
}

impl ToHuman for StaircaseStatus {
    fn to_human(&self) -> String {
        let mut out = format!("Staircase: {}\n", self.metadata.name);
        out.push_str(&format!("ID: {}\n", self.metadata.id));
        out.push_str(&format!("Target: {}\n", self.metadata.target));
        out.push_str(&format!("Clean: {}\n", self.is_clean));
        out.push_str("Steps:\n");
        for (i, step) in self.steps.iter().enumerate() {
            let meta_step = &self.metadata.steps[i];
            out.push_str(&format!("  Step {} ({}):", i + 1, step.name));
            if step.is_modified {
                out.push_str(" [MODIFIED]");
            }
            if step.is_stale {
                out.push_str(" [STALE]");
            }
            out.push_str("\n");
            out.push_str(&format!("    Expected Cut: {}\n", step.expected_cut));
            if let Some(ref act) = step.actual_oid {
                out.push_str(&format!("    Actual OID:   {}\n", act));
            } else {
                out.push_str("    Actual OID:   [MISSING BRANCH]\n");
            }
            if let Some(ref b) = meta_step.branch {
                out.push_str(&format!("    Branch:       {}\n", b));
            }
        }
        out
    }
}

impl ToHuman for StaircaseFamily {
    fn to_human(&self) -> String {
        let mut out = format!("  Name: {}\n", self.name);
        out.push_str(&format!("  ID: {}\n", self.id));
        out.push_str(&format!("  Target: {}\n", self.target));
        out.push_str(&format!("  Roots: {}\n", self.roots.join(", ")));
        out.push_str("  Steps:\n");
        for (name, step) in &self.steps {
            out.push_str(&format!("    Step {}:\n", name));
            out.push_str(&format!("      Cut: {}\n", step.cut));
            if let Some(ref b) = step.branch {
                out.push_str(&format!("      Branch: {}\n", b));
            }
            if !step.children.is_empty() {
                out.push_str(&format!("      Children: {}\n", step.children.join(", ")));
            }
        }
        out
    }
}


impl ToHuman for Discovery {
    fn to_human(&self) -> String {
        match self {
            Discovery::Linear(s) => s.to_human(),
            Discovery::Ambiguous(f) => f.to_human(),
        }
    }
}

impl ToHuman for VerificationResult {
    fn to_human(&self) -> String {
        let mut out = format!(
            "Step {}: {}\n",
            self.step_name,
            if self.success { "PASSED" } else { "FAILED" }
        );
        if !self.success {
            out.push_str(&format!("Stdout:\n{}\n", self.stdout));
            out.push_str(&format!("Stderr:\n{}\n", self.stderr));
        }
        out
    }
}

impl<T: ToHuman> ToHuman for Vec<T> {
    fn to_human(&self) -> String {
        self.iter()
            .map(|x| x.to_human())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

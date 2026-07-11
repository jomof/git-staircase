use crate::ResolvedStaircase;
use crate::model::{
    Discovery, StaircaseFamily, StaircaseMetadata, StaircaseStatus, Step, VerificationResult,
};
use serde::Serialize;

pub trait ToHuman {
    fn to_human(&self) -> String;
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
            self.state()
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
        if let Some(ref results) = self.verification_results {
            for result in results {
                out.push_str(&format!(
                    "verify\t{}\t{}\t{}\n",
                    result.step_name,
                    if result.success { "pass" } else { "fail" },
                    result.cut
                ));
            }
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
        let mut out = self.metadata.name.to_string();
        if self.is_implicit {
            out.push_str(" (implicit)");
        }
        out.push('\n');
        out.push_str(&format!("  target: {}\n", self.metadata.target));
        out.push_str(&format!("  state: {}\n", self.state()));
        out.push_str(&format!("  steps: {}\n", self.steps.len()));
        out.push_str(&format!(
            "  lineage: {}\n",
            if self.is_implicit {
                "none"
            } else {
                &self.metadata.id
            }
        ));

        if let Some(ref results) = self.verification_results {
            out.push_str("  verification:\n");
            for result in results {
                out.push_str(&format!(
                    "    {}: {}\n",
                    result.step_name,
                    if result.success { "PASS" } else { "FAIL" }
                ));
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

impl ToPorcelain for ResolvedStaircase {
    fn to_porcelain(&self) -> String {
        match self {
            ResolvedStaircase::ImplicitFamily(f) => {
                format!("{}\t{}\tfamily\t{}", f.name, f.id, f.steps.len())
            }
            _ => {
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
    }
}

impl ToHuman for ResolvedStaircase {
    fn to_human(&self) -> String {
        match self {
            ResolvedStaircase::ImplicitFamily(f) => {
                format!("Implicit Staircase Family: {}\n{}", f.name, f.to_human())
            }
            ResolvedStaircase::Managed(m) => {
                format!("Managed Staircase: {}\n{}", m.name, m.to_human())
            }
            ResolvedStaircase::Implicit(m) => {
                format!("Implicit Staircase: {}\n{}", m.name, m.to_human())
            }
        }
    }
}

#[derive(Serialize)]
pub struct Success {
    pub message: String,
}

impl Success {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ToHuman for Success {
    fn to_human(&self) -> String {
        self.message.clone()
    }
}

impl ToPorcelain for Success {
    fn to_porcelain(&self) -> String {
        String::new()
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct Summary<T>(pub T);

impl ToHuman for Summary<StaircaseStatus> {
    fn to_human(&self) -> String {
        let s = &self.0;
        let m = &s.metadata;
        let steps_count = m.steps.len();
        let steps_word = if steps_count == 1 { "step" } else { "steps" };
        let implicit_marker = if s.is_implicit { " (implicit)" } else { "" };
        format!(
            "{} {} {} {}{}",
            m.name,
            steps_count,
            steps_word,
            s.state(),
            implicit_marker
        )
    }
}

impl ToHuman for Summary<StaircaseFamily> {
    fn to_human(&self) -> String {
        let f = &self.0;
        let path_count = f.steps.values().filter(|s| s.children.is_empty()).count();
        let paths_word = if path_count == 1 { "path" } else { "paths" };
        format!("{} {} {} (implicit)", f.name, path_count, paths_word)
    }
}

impl ToPorcelain for Summary<StaircaseStatus> {
    fn to_porcelain(&self) -> String {
        self.0.to_porcelain()
    }
}

impl ToPorcelain for Summary<StaircaseFamily> {
    fn to_porcelain(&self) -> String {
        format!(
            "{}\t{}\tfamily\t{}",
            self.0.name,
            self.0.id,
            self.0.steps.len()
        )
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ReorderResult {
    pub status: StaircaseStatus,
}

impl ToHuman for ReorderResult {
    fn to_human(&self) -> String {
        "Reordered staircase.".to_string()
    }
}

impl ToPorcelain for ReorderResult {
    fn to_porcelain(&self) -> String {
        String::new()
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct StepsList(pub Vec<Step>);

impl ToHuman for StepsList {
    fn to_human(&self) -> String {
        self.0
            .iter()
            .enumerate()
            .map(|(i, step)| format!("Step {}: {} ({})", i + 1, step.name, &step.cut[..7]))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToPorcelain for StepsList {
    fn to_porcelain(&self) -> String {
        self.0
            .iter()
            .enumerate()
            .map(|(i, step)| format!("{}\t{}\t{}", i + 1, step.name, step.cut))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Serialize)]
pub struct StaircaseCommits {
    pub steps: Vec<StepCommits>,
}

#[derive(Serialize)]
pub struct StepCommits {
    pub index: usize,
    pub name: String,
    pub commits: Vec<CommitInfo>,
}

#[derive(Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub subject: String,
}

impl ToHuman for StaircaseCommits {
    fn to_human(&self) -> String {
        let mut out = String::new();
        for step in &self.steps {
            out.push_str(&format!("Step {}: {}\n", step.index, step.name));
            for commit in &step.commits {
                out.push_str(&format!("  {} {}\n", commit.hash, commit.subject));
            }
        }
        out
    }
}

impl ToPorcelain for StaircaseCommits {
    fn to_porcelain(&self) -> String {
        let mut out = String::new();
        for step in &self.steps {
            out.push_str(&format!("step\t{}\t{}\n", step.index, step.name));
            for commit in &step.commits {
                out.push_str(&format!("commit\t{}\t{}\n", commit.hash, commit.subject));
            }
        }
        out
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct PlainOutput(pub String);

impl ToHuman for PlainOutput {
    fn to_human(&self) -> String {
        self.0.clone()
    }
}

impl ToPorcelain for PlainOutput {
    fn to_porcelain(&self) -> String {
        self.0.clone()
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct LogOutput(pub Vec<CommitInfo>);

impl ToHuman for LogOutput {
    fn to_human(&self) -> String {
        self.0
            .iter()
            .map(|c| format!("{} {}", c.hash, c.subject))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToPorcelain for LogOutput {
    fn to_porcelain(&self) -> String {
        self.0
            .iter()
            .map(|c| format!("{}\t{}", c.hash, c.subject))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

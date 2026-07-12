use super::PresentationOutput;
use crate::GitRepo;
use crate::core;
use crate::model::{LandingPolicy, StaircaseMetadata, Step, VerificationPolicy};
use anyhow::{Context, Result, anyhow};
use uuid::Uuid;

#[derive(clap::Args, Clone, Debug)]
pub struct Adopt {
    pub name: String,
    #[arg(long)]
    pub onto: Option<String>,
    /// List of branch names in order (root to tip)
    pub branches: Vec<String>,
    #[arg(long)]
    pub build_command: Option<String>,
    #[arg(long)]
    pub test_command: Option<String>,
    #[arg(long)]
    pub landing_policy: Option<LandingPolicy>,
    #[arg(long)]
    pub verify_each_prefix: bool,
}

impl super::Command for Adopt {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        if self.branches.is_empty() {
            return Err(anyhow!("At least one branch must be specified to adopt"));
        }
        let mut steps = Vec::new();
        for b in &self.branches {
            let full_ref = if b.starts_with("refs/heads/") {
                b.clone()
            } else {
                format!("refs/heads/{}", b)
            };
            let oid = repo
                .resolve_commit(&full_ref)
                .with_context(|| format!("Failed to resolve branch '{}'", b))?;
            let short_name = b.strip_prefix("refs/heads/").unwrap_or(&b).to_string();
            steps.push(Step {
                id: String::new(),
                name: short_name.clone(),
                cut: oid,
                branch: Some(short_name),
            });
        }

        let verification_policy = if self.build_command.is_some() || self.test_command.is_some() {
            Some(VerificationPolicy {
                build_command: self.build_command.clone(),
                test_command: self.test_command.clone(),
                verify_each_prefix: self.verify_each_prefix,
            })
        } else {
            None
        };

        let target = match &self.onto {
            Some(o) => o.clone(),
            None => core::infer_onto(repo)?,
        };
        let staircase = StaircaseMetadata {
            landing_policy: self.landing_policy,
            id: Uuid::new_v4().to_string(),
            name: self.name.clone(),
            target,
            steps,
            verification_policy,

            primary_branch_layout: None,
            branch_layout_base: None,
        };

        let result = core::adopt(repo, &staircase)?;
        Ok(Box::new(result))
    }
}

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
        let mut staircase = if self.branches.is_empty() {
            let resolved = core::resolve_staircase(repo, &self.name, self.onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase selector '{}' not found", self.name))?;
            if resolved.staircase.is_managed() {
                return Err(anyhow!("Staircase '{}' is already managed", self.name));
            }
            let mut metadata = resolved.staircase.metadata().clone();
            if !self.name.starts_with("implicit@") {
                metadata.name = self.name.clone();
            }
            metadata
        } else {
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
                let short_name = b.strip_prefix("refs/heads/").unwrap_or(b).to_string();
                steps.push(Step {
                    id: String::new(),
                    name: short_name.clone(),
                    cut: oid,
                    branch: Some(short_name),
                });
            }

            let target = match &self.onto {
                Some(o) => o.clone(),
                None => core::infer_onto(repo)?,
            };
            StaircaseMetadata {
                landing_policy: self.landing_policy,
                id: Uuid::new_v4().to_string(),
                name: self.name.clone(),
                target,
                steps,
                verification_policy: None,

                primary_branch_layout: None,
                branch_layout_base: None,
                user_metadata: None,
                lifecycle: None,
            }
        };

        if let Some(lp) = self.landing_policy {
            staircase.landing_policy = Some(lp);
        }

        if self.build_command.is_some() || self.test_command.is_some() {
            staircase.verification_policy = Some(VerificationPolicy {
                build_command: self.build_command.clone().or_else(|| {
                    staircase
                        .verification_policy
                        .as_ref()
                        .and_then(|vp| vp.build_command.clone())
                }),
                test_command: self.test_command.clone().or_else(|| {
                    staircase
                        .verification_policy
                        .as_ref()
                        .and_then(|vp| vp.test_command.clone())
                }),
                verify_each_prefix: self.verify_each_prefix,
            });
        }

        let result = core::adopt(repo, &staircase)?;
        Ok(Box::new(result))
    }
}

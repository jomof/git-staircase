use super::{PresentationOutput, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use crate::model::{LandingPolicy, StaircaseMetadata, Step, VerificationPolicy};
use anyhow::{Context, Result, anyhow};
use uuid::Uuid;

#[derive(clap::Args, Clone, Debug)]
pub struct Adopt {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    /// Optional new name for the adopted staircase.
    #[arg(long("name"))]
    pub rename: Option<String>,
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
            let resolved = self.selector.resolve(repo)?;
            let mut metadata = resolved.metadata().clone();
            if let Some(new_name) = &self.rename {
                metadata.name = new_name.clone();
            }
            metadata
        } else {
            let name = self
                .rename
                .clone()
                .or_else(|| self.selector.base.name.clone())
                .ok_or_else(|| anyhow!("A name must be provided for the staircase"))?;

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

            let target = match &self.selector.base.onto {
                Some(o) => o.clone(),
                None => core::infer_onto(repo)?,
            };

            StaircaseMetadata {
                landing_policy: self.landing_policy,
                id: Uuid::new_v4().to_string(),
                name,
                symbolic_integration_target: target,
                steps,
                verification_policy: None,

                primary_branch_layout: None,
                branch_layout_base: None,
                user_metadata: None,
                lifecycle: None,
            }
        };

        // Override metadata with explicit flags if provided
        if let Some(onto) = &self.selector.base.onto {
            staircase.symbolic_integration_target = onto.clone();
        }
        if let Some(lp) = self.landing_policy {
            staircase.landing_policy = Some(lp);
        }
        if self.build_command.is_some() || self.test_command.is_some() {
            let mut vp = staircase.verification_policy.unwrap_or(VerificationPolicy {
                build_command: None,
                test_command: None,
                verify_each_prefix: false,
            });
            if let Some(bc) = &self.build_command {
                vp.build_command = Some(bc.clone());
            }
            if let Some(tc) = &self.test_command {
                vp.test_command = Some(tc.clone());
            }
            if self.verify_each_prefix {
                vp.verify_each_prefix = true;
            }
            staircase.verification_policy = Some(vp);
        }

        let result = core::adopt(repo, &staircase)?;
        Ok(Box::new(result))
    }
}

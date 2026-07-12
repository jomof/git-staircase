use super::PresentationOutput;
use crate::GitRepo;
use crate::core;
use crate::model::{StaircaseMetadata, Step, VerificationPolicy};
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
    pub verify_each_prefix: bool,
}

impl super::Command for Adopt {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(
            repo,
            self.name.clone(),
            self.onto.clone(),
            self.branches.clone(),
            self.build_command.clone(),
            self.test_command.clone(),
            self.verify_each_prefix,
        )?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    name: String,
    onto: Option<String>,
    branches: Vec<String>,
    build_command: Option<String>,
    test_command: Option<String>,
    verify_each_prefix: bool,
) -> Result<StaircaseMetadata> {
    if branches.is_empty() {
        return Err(anyhow!("At least one branch must be specified to adopt"));
    }
    let mut steps = Vec::new();
    for b in branches {
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

    let verification_policy = if build_command.is_some() || test_command.is_some() {
        Some(VerificationPolicy {
            build_command,
            test_command,
            verify_each_prefix,
        })
    } else {
        None
    };

    let target = match onto {
        Some(o) => o,
        None => core::infer_onto(repo)?,
    };
    let staircase = StaircaseMetadata {
        id: Uuid::new_v4().to_string(),
        name: name.clone(),
        target,
        steps,
        verification_policy,

        primary_branch_layout: None,
        branch_layout_base: None,
    };

    Ok(core::adopt(repo, &staircase)?)
}

pub fn run(
    repo: &GitRepo,
    _format: super::OutputFormat,
    name: String,
    onto: Option<String>,
    branches: Vec<String>,
    build_command: Option<String>,
    test_command: Option<String>,
    verify_each_prefix: bool,
) -> Result<()> {
    let result = run_internal(
        repo,
        name,
        onto,
        branches,
        build_command,
        test_command,
        verify_each_prefix,
    )?;
    println!("Adopted staircase '{}' (ID: {}).", result.name, result.id);
    Ok(())
}

use super::{OutputFormat, print_output};
use crate::GitRepo;
use anyhow::{Context, anyhow};
use git_staircase::core;
use git_staircase::model::{StaircaseMetadata, Step, VerificationPolicy};

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: String,
    onto: Option<String>,
    branches: Vec<String>,
    build_command: Option<String>,
    test_command: Option<String>,
    verify_each_prefix: bool,
) -> anyhow::Result<()> {
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
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        target,
        steps,
        verification_policy,
    };

    core::adopt(repo, &staircase)?;

    if matches!(format, OutputFormat::Human) {
        println!("Adopted staircase '{}' (ID: {}).", name, staircase.id);
        Ok(())
    } else {
        print_output(format, &staircase)
    }
}

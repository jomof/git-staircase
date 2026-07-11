pub mod cli;
pub mod core;
pub mod error;
pub mod git;
pub mod model;

pub use crate::core::ResolvedStaircase;
pub use error::{Result, StaircaseError};
pub use git::GitRepo;
pub use model::{
    Discovery, FamilyStep, IdentityKind, StaircaseFamily, StaircaseMetadata, StaircaseStatus, Step,
    StepStatus, ToHuman, ToPorcelain, VerificationPolicy, VerificationResult,
};

pub fn parse_step_spec(spec: &str) -> anyhow::Result<(String, usize)> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid step spec '{}'. Expected format: <staircase_name>:<step_number>",
            spec
        ));
    }
    let name = parts[0].to_string();
    let num = parts[1]
        .parse::<usize>()
        .map_err(|e| anyhow::anyhow!("Failed to parse step number: {}", e))?;
    Ok((name, num))
}

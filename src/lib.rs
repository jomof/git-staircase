pub mod cli;
pub mod core;
pub mod error;
pub mod git;
pub mod model;

pub use crate::core::{ResolvedSelector, ResolvedStaircase};
pub use cli::formatting::{ToHuman, ToPorcelain};
pub use error::{Result, StaircaseError};
pub use git::GitRepo;
pub use model::{
    Discovery, FamilyStep, IdentityKind, StaircaseFamily, StaircaseMetadata, StaircaseStatus, Step,
    StepStatus, VerificationPolicy, VerificationResult,
};

pub fn parse_step_spec(spec: &str) -> anyhow::Result<(String, usize)> {
    let (name, num_str) = spec.rsplit_once(":").ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid step spec \"{}\". Expected format: <staircase_name>:<step_number>",
            spec
        )
    })?;
    let num = num_str
        .parse::<usize>()
        .map_err(|e| anyhow::anyhow!("Failed to parse step number \"{}\": {}", num_str, e))?;
    Ok((name.to_string(), num))
}

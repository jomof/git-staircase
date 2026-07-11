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

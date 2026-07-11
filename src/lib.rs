pub mod core;
pub mod error;
pub mod git;
pub mod model;

pub use error::{Result, StaircaseError};
pub use git::GitRepo;
pub use model::{
    Discovery, FamilyStep, StaircaseFamily, StaircaseMetadata, StaircaseStatus, Step, StepStatus,
};

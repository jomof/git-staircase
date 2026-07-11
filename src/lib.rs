pub mod error;
pub mod model;
pub mod git;
pub mod core;

pub use error::{StaircaseError, Result};
pub use model::{StaircaseMetadata, Step, StepStatus, StaircaseStatus};
pub use git::GitRepo;

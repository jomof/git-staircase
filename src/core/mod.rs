pub mod discovery;
pub mod identity;
pub mod inference;
pub mod manipulation;
pub mod persistence;
pub mod resolved;
pub mod status;
pub mod utils;
pub mod verification;

pub use discovery::{
    discover, find_by_name, resolve_by_id, resolve_by_name, resolve_by_ref, resolve_by_revision,
    resolve_by_structural_key, resolve_explicit_staircase, resolve_staircase,
};
pub use identity::compute_identity;
pub use inference::infer_onto;
pub use manipulation::{delete, drop, join, move_commits, rebase, reorder, restack, split};
pub use resolved::ResolvedStaircase;
pub use resolved::adopt;
pub use status::{get_status, get_status_metadata};
pub use verification::verify;

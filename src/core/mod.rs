pub mod discovery;
pub mod graph;
pub mod identity;
pub mod inference;
pub mod manipulation;
pub mod persistence;
pub mod resolution;
pub mod resolved;
pub mod status;
pub mod utils;
pub mod verification;

pub use discovery::discover;
pub use identity::compute_identity;
pub use inference::infer_onto;
pub use manipulation::{ 
    delete, drop, join, move_commits, rebase, reorder, restack, split, 
    DropOptions, JoinOptions, JoinRefAction, RebaseOptions, ReorderOptions, SplitOptions 
};
pub use resolution::{
    find_by_name, resolve_by_id, resolve_by_name, resolve_by_ref, resolve_by_revision,
    resolve_by_structural_key, resolve_explicit_staircase, resolve_staircase,
};
pub use resolved::adopt;
pub use resolved::{ResolvedSelector, ResolvedStaircase};
pub use status::{get_status, get_status_metadata};
pub use verification::verify;

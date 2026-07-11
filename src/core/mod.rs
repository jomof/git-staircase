pub mod discovery;
pub mod manipulation;
pub mod status;
pub mod verification;

pub use discovery::{
    compute_identity, discover, find_by_name, infer_onto, resolve_explicit_staircase,
    resolve_staircase,
};
pub use manipulation::{adopt, delete, drop, join, move_commits, rebase, reorder, restack, split};
pub use status::{get_status, get_status_metadata};
pub use verification::verify;

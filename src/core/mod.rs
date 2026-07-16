pub mod archive;
pub mod discovery;
pub mod draft;
pub mod graph;
pub mod identity;
pub mod inference;
pub mod list;
pub mod local;
pub mod manipulation;
pub mod metadata;
pub mod operation;
pub mod persistence;
pub mod refs;
#[cfg(test)]
pub mod refs_test;
pub mod resolution;
pub mod resolved;
pub mod restack;
pub(crate) mod rewrite;
pub mod status;
pub mod unarchive;
pub mod utils;
pub mod verification;

pub use archive::{ArchiveOptions, ArchiveResult, archive_staircase, release_staircase_name};
pub use discovery::discover;
pub use draft::{
    DraftDiffMode, MaterializeOptions, MaterializeResult, attach_draft, create_snapshot,
    detach_draft, diff_draft, get_worktree_draft, materialize_draft, restore_snapshot,
};
pub use identity::compute_identity;
pub use inference::infer_onto;
pub use list::{ListFilter, list};
pub use local::{
    DiscoveryOverride, LayoutBranch, LayoutState, LocalMutationResult, add_discovery_override,
    append, assign_step_branch, clear_discovery_override, discovery_overrides, layout_state,
    name_staircase, normalize, policy_values, rename_staircase, set_layout, unname_staircase,
    unset_layout, update_policies,
};
pub use manipulation::{
    DropOptions, JoinOptions, JoinRefAction, LandOptions, RebaseOptions, ReorderOptions,
    SplitOptions, delete, drop, drop_with_dry_run, join, land, land_through, move_commits,
    move_commits_with_dry_run, rebase, rebase_with_dry_run, reorder, reorder_dry_run, restack,
    restack_from, split, validate_split, validate_split_plan,
};
pub use metadata::{
    add_label, add_link, get_step_metadata, get_step_metadata_snapshot, get_user_metadata,
    get_user_metadata_snapshot, remove_label, set_description, set_title, update_step_metadata,
    update_step_metadata_expected, update_user_metadata, update_user_metadata_expected,
};
pub use operation::{
    DraftRecovery, MutationPlan, OperationJournal, OperationPhase, OperationResult, RefMutation,
    abort_active, active_operation, continue_active, ensure_no_active, external_git_operation,
};
pub use persistence::{
    list_all_staircases, list_archived_staircases, list_staircases, read_record, write_record,
};
pub use resolution::{
    find_by_name, resolve_by_id, resolve_by_name, resolve_by_record, resolve_by_ref,
    resolve_by_revision, resolve_by_structural_key, resolve_explicit_staircase, resolve_staircase,
};
pub use resolved::adopt;
pub use resolved::{ResolvedSelector, ResolvedStaircase};
pub use restack::{RestackOptions, RestackStrategy, Restacker};
pub use status::{get_status, get_status_metadata};
pub use unarchive::{
    UnarchiveBranchesMode, UnarchiveOptions, UnarchiveResult, unarchive_staircase,
};
pub use verification::{DraftVerificationEvidence, verify, verify_draft};

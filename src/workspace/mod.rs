pub mod bootstrap;
pub mod doctor;
pub mod gerrit_provider;
pub mod model;
pub mod provider;
pub mod repo_provider;
pub mod storage;

pub use bootstrap::{bootstrap, BootstrapOptions, BootstrapResult};
pub use doctor::{doctor, WorkspaceDoctorReport};
pub use gerrit_provider::{get_gerrit_descriptor, parse_change_ids, probe_gerrit_route};
pub use model::{
    BindingProvenance, Capability, CapabilityBinding, ProviderDescriptor, WorkspaceCandidate,
    WorkspaceRecord,
};
pub use provider::{discover_installed_providers, expand_profile};
pub use repo_provider::{get_repo_descriptor, probe_repo_workspace};
pub use storage::{
    find_workspace_record_for_path, forget_workspace_record, list_workspace_records,
    load_workspace_record_by_id, save_workspace_record,
};

pub mod bootstrap;
pub mod doctor;
pub mod model;
pub mod provider;
pub mod storage;

pub use bootstrap::{bootstrap, BootstrapOptions, BootstrapResult};
pub use doctor::{doctor, WorkspaceDoctorReport};
pub use model::{
    BindingProvenance, Capability, CapabilityBinding, ProviderDescriptor, WorkspaceCandidate,
    WorkspaceRecord,
};
pub use provider::{discover_installed_providers, expand_profile};
pub use storage::{
    find_workspace_record_for_path, forget_workspace_record, list_workspace_records,
    load_workspace_record_by_id, save_workspace_record,
};

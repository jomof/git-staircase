pub mod bootstrap;
pub mod doctor;
pub mod gerrit_provider;
pub mod github_provider;
pub mod model;
pub mod provider;
pub mod provider_base;
pub mod provider_utils;
#[cfg(test)]
mod provider_utils_test;
pub mod repo_provider;
pub mod review_provider;
#[cfg(test)]
mod review_provider_test;
pub mod storage;

pub use bootstrap::{BootstrapOptions, BootstrapResult, bootstrap};
pub use doctor::{WorkspaceDoctorReport, doctor};
pub use gerrit_provider::{
    GerritProvider, get_gerrit_descriptor, parse_change_ids, probe_gerrit_route,
};
pub use github_provider::{
    GitHubProvider, get_github_descriptor, parse_github_remote_url, probe_github_route,
};
pub use model::{
    BindingProvenance, Capability, CapabilityBinding, CapabilityReadiness, EvidenceAuthority,
    ProviderDescriptor, TypedEvidence, WorkspaceCandidate, WorkspaceHint, WorkspaceRecord,
};
pub use provider::{discover_installed_providers, expand_profile};
pub use provider_utils::{GitUrlInfo, parse_git_url};
pub use repo_provider::{
    ProductionRepoMetadataSource, RepoCheckoutEvidence, RepoDiscoveryReport, RepoForallInvocation,
    RepoForallMetadata, RepoMetadataSource, RepoProjectMapping, RepoRefresh,
    controlled_repo_forall_invocation, get_repo_descriptor, observe_repo_workspace,
    observe_repo_workspace_with_source, parse_repo_forall_output, probe_repo_workspace,
    probe_repo_workspace_with_source, refresh_repo_workspace,
};
pub use review_provider::{
    FakeTransport, OperationJournal, ProductionTransport, ProviderTransport, ReviewProvider,
    ReviewProviderInstance, SynchronizationState, TransportRequest, TransportResponse,
    prepare_review_state, publish_provider_extension_cas,
};
pub use storage::{
    find_workspace_record_for_path, forget_workspace_record, list_workspace_records,
    load_workspace_record_by_id, save_workspace_record, save_workspace_record_cas,
};

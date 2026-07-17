use git_staircase::workspace::model::WorkspaceRecord;
use git_staircase::workspace::storage::*;
use std::collections::HashMap;
use std::fs;

#[test]
fn test_list_records_ignores_corrupted_json() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage_dir = tmp.path().join("workspaces");
    fs::create_dir_all(&storage_dir).unwrap();
    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", &storage_dir);
    }

    // Create one valid record
    let record = WorkspaceRecord {
        workspace_id: "valid".to_string(),
        canonical_root: tmp.path().to_path_buf(),
        provider_native_key: None,
        capability_bindings: HashMap::new(),
        binding_provenance: HashMap::new(),
        capability_readiness: HashMap::new(),
        discovery_fingerprint: HashMap::new(),
        last_successful_validation: 0,
        current_project_id: None,
        generation: 0,
        extensions: HashMap::new(),
    };
    save_workspace_record(&record).unwrap();

    // Create one corrupted record
    fs::write(storage_dir.join("corrupted.json"), "{ invalid json }").unwrap();

    let result = list_workspace_records();
    assert!(
        result.is_err(),
        "Should have returned an error due to corrupted JSON"
    );
}

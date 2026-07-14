use git_staircase::workspace::model::WorkspaceRecord;
use git_staircase::workspace::storage::{
    find_workspace_record_for_path, save_workspace_record_cas,
};
use std::collections::HashMap;
use std::time::Instant;

#[test]
fn test_storage_performance() {
    let mut record = WorkspaceRecord {
        workspace_id: String::new(),
        canonical_root: std::env::current_dir().unwrap(),
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

    for i in 0..100 {
        record.workspace_id = format!("perf-test-{}", i);
        record.canonical_root = std::env::current_dir().unwrap().join(format!("dir-{}", i));
        save_workspace_record_cas(&record, None).unwrap();
    }

    let start = Instant::now();
    for _ in 0..10 {
        find_workspace_record_for_path(&std::env::current_dir().unwrap()).unwrap();
    }
    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 100,
        "find_workspace_record_for_path is too slow: {:?}",
        duration
    );
}

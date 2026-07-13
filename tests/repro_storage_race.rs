use git_staircase::workspace::model::WorkspaceRecord;
use git_staircase::workspace::storage::save_workspace_record;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

#[test]
fn test_storage_race() {
    // ARRANGE: Prepare a record and multiple threads to save it
    let workspace_id = "test-race-workspace";
    let record = WorkspaceRecord {
        workspace_id: workspace_id.to_string(),
        canonical_root: std::path::PathBuf::from("/tmp"),
        provider_native_key: None,
        capability_bindings: HashMap::new(),
        binding_provenance: HashMap::new(),
        discovery_fingerprint: HashMap::new(),
        last_successful_validation: 0,
        current_project_id: None,
    };

    let record = Arc::new(record);
    let mut handles = Vec::new();

    // ACT: Simultaneously save the same record from many threads
    for _ in 0..20 {
        let rec = Arc::clone(&record);
        handles.push(thread::spawn(move || {
            let mut failed = 0;
            for _ in 0..100 {
                if save_workspace_record(&rec).is_err() {
                    failed += 1;
                }
            }
            failed
        }));
    }

    let mut total_failed = 0;
    for h in handles {
        total_failed += h.join().unwrap();
    }

    // ASSERT: Verify that at least one save operation failed due to the race (this currently SUCCEEDS in reproducing the failure)
    assert!(
        total_failed > 0,
        "Expected at least one save operation to fail due to race condition, but all succeeded! Total failed: {}",
        total_failed
    );
}

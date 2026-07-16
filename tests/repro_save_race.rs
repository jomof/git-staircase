use git_staircase::workspace::model::WorkspaceRecord;
use git_staircase::workspace::storage::{save_workspace_record, save_workspace_record_cas};
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

#[test]
fn test_save_workspace_record_race_condition() {
    // ARRANGE
    let tmp = TempDir::new().unwrap();
    let storage_dir = tmp.path().to_path_buf();

    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", &storage_dir);
    }

    let workspace_id = "race-test-ws".to_string();
    let record = WorkspaceRecord {
        workspace_id: workspace_id.clone(),
        canonical_root: PathBuf::from("/tmp/test"),
        provider_native_key: None,
        capability_bindings: std::collections::HashMap::new(),
        binding_provenance: std::collections::HashMap::new(),
        capability_readiness: std::collections::HashMap::new(),
        discovery_fingerprint: std::collections::HashMap::new(),
        last_successful_validation: 0,
        current_project_id: None,
        generation: 0,
        extensions: std::collections::HashMap::new(),
    };

    // First save to establish the record
    save_workspace_record(&record).unwrap();

    let num_threads = 100;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    // ACT: Multiple threads try to update the record concurrently.
    for i in 0..num_threads {
        let b = barrier.clone();
        let mut r = record.clone();
        r.generation = 1; // Expected generation for CAS
        r.current_project_id = Some(format!("thread-{}", i));

        handles.push(thread::spawn(move || {
            b.wait();
            save_workspace_record_cas(&r, Some(1))
        }));
    }

    let mut successes = 0;
    for h in handles {
        if h.join().unwrap().is_ok() {
            successes += 1;
        }
    }

    // ASSERT
    // If CAS is working correctly and atomic, only 1 should succeed.
    assert_eq!(
        successes, 1,
        "Expected only one thread to succeed in CAS update, but {} succeeded",
        successes
    );
}

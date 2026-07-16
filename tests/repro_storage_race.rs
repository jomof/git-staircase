use git_staircase::workspace::model::WorkspaceRecord;
use git_staircase::workspace::storage::*;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

#[test]
fn test_storage_race_condition() {
    let storage_tmp = TempDir::new().unwrap();
    // Use environment variable to point to our temp storage
    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_tmp.path());
    }

    let workspace_id = "test-race";
    let record = WorkspaceRecord {
        workspace_id: workspace_id.to_string(),
        canonical_root: storage_tmp.path().to_path_buf(),
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

    // Initial save
    save_workspace_record(&record).unwrap();

    let num_threads = 10;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let b = barrier.clone();
        let wid = workspace_id.to_string();
        handles.push(thread::spawn(move || {
            b.wait();
            // Try to update the record multiple times
            for _ in 0..10 {
                loop {
                    if let Ok(Some(rec)) = load_workspace_record_by_id(&wid) {
                        let expected_gen = rec.generation;
                        let mut updated = rec.clone();
                        updated
                            .extensions
                            .insert("update".to_string(), serde_json::json!("done"));
                        if save_workspace_record_cas(&updated, Some(expected_gen)).is_ok() {
                            break;
                        }
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_record = load_workspace_record_by_id(workspace_id).unwrap().unwrap();
    println!("Final generation: {}", final_record.generation);
    assert_eq!(
        final_record.generation, 101,
        "Updates were lost due to race condition!"
    );
}

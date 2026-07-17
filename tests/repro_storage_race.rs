use git_staircase::workspace::model::WorkspaceRecord;
use git_staircase::workspace::storage::{
    load_workspace_record_by_id, save_workspace_record, save_workspace_record_cas,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

#[test]
fn test_storage_race_condition() {
    let storage_dir = tempfile::TempDir::new().unwrap();
    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_dir.path());
    }

    let workspace_id = "test-ws";
    let record = WorkspaceRecord {
        workspace_id: workspace_id.to_string(),
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

    // Initial save
    save_workspace_record(&record).unwrap();

    let _record = Arc::new(record);
    let mut handles = Vec::new();

    for i in 0..10 {
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                if let Ok(Some(current)) = load_workspace_record_by_id("test-ws") {
                    let mut updated = current.clone();
                    updated
                        .extensions
                        .insert(format!("thread-{}", i), "value".into());
                    // This SHOULD fail if someone else updated it in between,
                    // because save_workspace_record_cas checks the generation.
                    // But if the check-and-save is not atomic, it might succeed when it shouldn't,
                    // or overwrite other's changes.
                    let _ = save_workspace_record_cas(&updated, Some(current.generation));
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_record = load_workspace_record_by_id("test-ws").unwrap().unwrap();
    println!("Final generation: {}", final_record.generation);
    println!("Extensions count: {}", final_record.extensions.len());

    // We expect 10 extensions if every thread succeeded at least once and didn't overwrite others.
    // In a race, it's very likely some will be lost.
    assert_eq!(
        final_record.extensions.len(),
        10,
        "Updates were lost due to race condition!"
    );
}

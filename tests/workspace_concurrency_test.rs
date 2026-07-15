use git_staircase::workspace::{
    WorkspaceRecord, load_workspace_record_by_id, save_workspace_record_cas,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::TempDir;

#[test]
fn test_concurrent_workspace_updates_atomic() {
    // ARRANGE: Create a workspace record.
    let storage_dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_dir.path());
    }

    let workspace_id = "test-ws".to_string();
    let initial_record = WorkspaceRecord {
        workspace_id: workspace_id.clone(),
        canonical_root: PathBuf::from("/tmp/test"),
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

    save_workspace_record_cas(&initial_record, None).expect("Initial save failed");

    let num_threads = 10;
    let mut handles = vec![];
    let success_count = Arc::new(Mutex::new(0));

    // ACT: Spawn 10 concurrent threads, each attempting to update a different extension field
    // in the same record using save_workspace_record_cas.
    for i in 0..num_threads {
        let ws_id = workspace_id.clone();
        let s_count = Arc::clone(&success_count);

        handles.push(thread::spawn(move || {
            let mut retries = 0;
            while retries < 100 {
                let record = load_workspace_record_by_id(&ws_id).unwrap().unwrap();
                let expected_gen = record.generation;
                let mut updated = record.clone();
                updated
                    .extensions
                    .insert(format!("thread-{}", i), serde_json::Value::from(i));

                match save_workspace_record_cas(&updated, Some(expected_gen)) {
                    Ok(_) => {
                        *s_count.lock().unwrap() += 1;
                        return;
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        if err_msg.contains("concurrent-workspace-update") {
                            retries += 1;
                            thread::yield_now();
                            continue;
                        } else {
                            panic!("Unexpected error in thread {}: {}", i, e);
                        }
                    }
                }
            }
            panic!("Thread {} failed to update after 100 retries", i);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // ASSERT: Verify that only one update succeeds for the same generation (enforced by CAS)
    // and that all updates eventually succeed without losing any data.
    let final_record = load_workspace_record_by_id(&workspace_id).unwrap().unwrap();

    assert_eq!(
        final_record.extensions.len(),
        num_threads,
        "Updates were lost!"
    );
    assert_eq!(*success_count.lock().unwrap(), num_threads);
    assert_eq!(final_record.generation, (num_threads + 1) as u64);
}

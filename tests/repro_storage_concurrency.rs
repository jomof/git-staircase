use git_staircase::workspace::model::WorkspaceRecord;
use git_staircase::workspace::storage::{load_workspace_record_by_id, save_workspace_record};
use std::collections::HashMap;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_concurrent_storage_update_leak() {
    let storage_dir = tempfile::TempDir::new().unwrap();
    unsafe {
        std::env::set_var("GIT_STAIRCASE_WORKSPACE_DIR", storage_dir.path());
    }

    let ws_id = "test-ws".to_string();
    let record = WorkspaceRecord {
        workspace_id: ws_id.clone(),
        canonical_root: std::path::PathBuf::from("/tmp"),
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

    let num_threads = 10;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    for i in 0..num_threads {
        let b = barrier.clone();
        let ws_id_clone = ws_id.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            let mut rec = load_workspace_record_by_id(&ws_id_clone).unwrap().unwrap();
            rec.extensions
                .insert(format!("thread-{}", i), "done".into());
            save_workspace_record(&rec).unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_record = load_workspace_record_by_id(&ws_id).unwrap().unwrap();
    println!("Final record extensions: {}", final_record.extensions.len());
    assert!(
        final_record.extensions.len() < num_threads as usize,
        "Expected lost updates but got {}",
        final_record.extensions.len()
    );
}

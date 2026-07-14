use git_staircase::workspace::storage::{get_workspace_storage_dir, list_workspace_records};
use std::fs;

#[test]
fn test_storage_silent_failure() {
    let storage_dir = get_workspace_storage_dir();
    fs::create_dir_all(&storage_dir).unwrap();
    let corrupted_path = storage_dir.join("corrupted.json");
    fs::write(&corrupted_path, "{ \"invalid\": json }").unwrap();

    let records = list_workspace_records().unwrap();
    let files_count = fs::read_dir(&storage_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .filter(|e| !e.file_name().to_str().unwrap().starts_with(".tmp_"))
        .count();

    assert_eq!(
        records.len(),
        files_count,
        "All JSON records should be accounted for; silent failure detected"
    );
}

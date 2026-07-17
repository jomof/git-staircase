use crate::error::{Result, StaircaseError};
use crate::workspace::model::WorkspaceRecord;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

thread_local! {
    static OVERRIDE_STORAGE_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub struct StorageDirGuard {
    prev: Option<PathBuf>,
}

impl Drop for StorageDirGuard {
    fn drop(&mut self) {
        OVERRIDE_STORAGE_DIR.with(|dir| {
            *dir.borrow_mut() = self.prev.take();
        });
    }
}

pub fn set_thread_storage_dir(path: &Path) -> StorageDirGuard {
    let _ = fs::create_dir_all(path);
    let prev = OVERRIDE_STORAGE_DIR.with(|dir| dir.borrow_mut().replace(path.to_path_buf()));
    StorageDirGuard { prev }
}

pub fn get_workspace_storage_dir() -> PathBuf {
    if let Some(override_dir) = OVERRIDE_STORAGE_DIR.with(|dir| dir.borrow().clone()) {
        if override_dir.exists() {
            return override_dir;
        } else if let Some(parent) = override_dir.parent() {
            if parent.exists() && fs::create_dir_all(&override_dir).is_ok() {
                return override_dir;
            }
        }
        OVERRIDE_STORAGE_DIR.with(|dir| *dir.borrow_mut() = None);
    }
    if cfg!(test) {
        thread_local! {
            static AUTO_THREAD_DIR: PathBuf = std::env::temp_dir().join(format!("git_staircase_test_{:?}", std::thread::current().id()));
        }
        return AUTO_THREAD_DIR.with(|dir| dir.clone());
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("git-staircase").join("workspaces")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("git-staircase")
            .join("workspaces")
    } else {
        std::env::temp_dir()
            .join("git-staircase")
            .join("workspaces")
    }
}

pub fn save_workspace_record(record: &WorkspaceRecord) -> Result<()> {
    save_workspace_record_cas(record, None)
}

/// Atomically publishes a workspace record after checking its complete generation.
///
/// `expected_generation` is required for updates made by provider commands. A
/// newly discovered record may pass `None`; an existing record then causes a
/// concurrent-update error instead of being overwritten.
pub fn save_workspace_record_cas(
    record: &WorkspaceRecord,
    expected_generation: Option<u64>,
) -> Result<()> {
    save_workspace_record_cas_at(record, expected_generation, &get_workspace_storage_dir())
}

pub fn save_workspace_record_for_repo(
    repo: &crate::GitRepo,
    record: &WorkspaceRecord,
) -> Result<()> {
    save_workspace_record_cas_at(record, None, &repo.get_storage_dir())
}

pub fn save_workspace_record_cas_at(
    record: &WorkspaceRecord,
    expected_generation: Option<u64>,
    dir: &Path,
) -> Result<()> {
    fs::create_dir_all(dir)?;

    let filename = format!("{}.json", record.workspace_id);
    let target_path = dir.join(&filename);
    let lock_path = dir.join(format!("{}.lock", record.workspace_id));

    let open = || {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
    };
    let mut attempts = 0;
    let mut _lock_file = loop {
        match open() {
            Ok(file) => break file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                attempts += 1;
                if attempts > 1000 {
                    let _ = fs::remove_file(&lock_path);
                    attempts = 0;
                    continue;
                }
                let existing_text = match fs::read_to_string(&lock_path) {
                    Ok(t) => t,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(StaircaseError::Io(e)),
                };
                let existing = serde_json::from_str::<serde_json::Value>(&existing_text).ok();
                let pid = existing
                    .as_ref()
                    .and_then(|lock| lock.get("pid"))
                    .and_then(|pid| pid.as_u64());
                let live = pid
                    .map(|pid| {
                        std::process::Command::new("kill")
                            .args(["-0", &pid.to_string()])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status()
                            .is_ok_and(|status| status.success())
                    })
                    .unwrap_or(true);
                if live && pid != Some(std::process::id() as u64) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                let _ = fs::remove_file(&lock_path);
                continue;
            }
            Err(error) => return Err(StaircaseError::Io(error)),
        }
    };
    use std::io::Write;
    let lock_data = serde_json::json!({
        "schema": "git-staircase/workspace-lock",
        "version": 1,
        "pid": std::process::id()
    });
    let _ = _lock_file.write_all(serde_json::to_string(&lock_data).unwrap().as_bytes());
    let _ = _lock_file.sync_all();

    struct LockGuard(std::path::PathBuf);
    impl Drop for LockGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let _guard = LockGuard(lock_path);

    let temp_path = dir.join(format!(
        ".tmp_{}_{}.json",
        record.workspace_id,
        uuid::Uuid::new_v4()
    ));

    let existing = if target_path.exists() {
        let data = fs::read_to_string(&target_path)?;
        Some(serde_json::from_str::<WorkspaceRecord>(&data).map_err(|e| {
            StaircaseError::Other(format!("Failed to parse workspace record: {}", e))
        })?)
    } else {
        None
    };
    match (existing.as_ref(), expected_generation) {
        (Some(current), Some(expected)) if current.generation != expected => {
            return Err(StaircaseError::Other(format!(
                "concurrent-workspace-update: expected generation {}, found {}",
                expected, current.generation
            )));
        }
        (Some(_), None) if record.generation == 0 => {
            return Err(StaircaseError::Other(
                "concurrent-workspace-update: record already exists".into(),
            ));
        }
        (None, Some(_)) => {
            return Err(StaircaseError::Other(
                "concurrent-workspace-update: record disappeared".into(),
            ));
        }
        _ => {}
    }

    let mut published = record.clone();
    published.generation = existing
        .as_ref()
        .map(|current| current.generation.saturating_add(1))
        .unwrap_or(1);

    let json_data = serde_json::to_string_pretty(&published).map_err(|e| {
        StaircaseError::Other(format!("Failed to serialize workspace record: {}", e))
    })?;

    fs::write(&temp_path, json_data)?;
    fs::rename(&temp_path, &target_path)?;

    Ok(())
}

pub fn load_workspace_record_by_id_at(workspace_id: &str, dir: &Path) -> Result<Option<WorkspaceRecord>> {
    let file_path = dir.join(format!("{}.json", workspace_id));
    if !file_path.exists() {
        return Ok(None);
    }
    let data = match fs::read_to_string(&file_path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let record: WorkspaceRecord = serde_json::from_str(&data)
        .map_err(|e| StaircaseError::Other(format!("Failed to parse workspace record: {}", e)))?;
    Ok(Some(record))
}

pub fn load_workspace_record_by_id(workspace_id: &str) -> Result<Option<WorkspaceRecord>> {
    load_workspace_record_by_id_at(workspace_id, &get_workspace_storage_dir())
}

pub fn load_workspace_record_by_id_for_repo(repo: &crate::GitRepo, workspace_id: &str) -> Result<Option<WorkspaceRecord>> {
    load_workspace_record_by_id_at(workspace_id, &repo.get_storage_dir())
}

pub fn list_workspace_records_at(dir: &Path) -> Result<Vec<WorkspaceRecord>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    for entry in entries {
        let Ok(entry) = entry else { continue; };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.starts_with(".tmp_") {
                    continue;
                }
            }
            let data = match fs::read_to_string(&path) {
                Ok(d) => d,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            let record: WorkspaceRecord = serde_json::from_str(&data).map_err(|e| {
                StaircaseError::Other(format!(
                    "Invalid storage record {}: {}",
                    path.display(),
                    e
                ))
            })?;
            records.push(record);
        }
    }
    Ok(records)
}

pub fn list_workspace_records() -> Result<Vec<WorkspaceRecord>> {
    list_workspace_records_at(&get_workspace_storage_dir())
}

pub fn list_workspace_records_for_repo(repo: &crate::GitRepo) -> Result<Vec<WorkspaceRecord>> {
    list_workspace_records_at(&repo.get_storage_dir())
}

pub fn find_workspace_record_for_repo(repo: &crate::GitRepo) -> Result<Option<WorkspaceRecord>> {
    find_workspace_record_for_path_at(&repo.workdir, &repo.get_storage_dir())
}

pub fn find_workspace_record_for_path_at(path: &Path, storage_dir: &Path) -> Result<Option<WorkspaceRecord>> {
    let records = list_workspace_records_at(storage_dir)?;
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    let mut best_match: Option<WorkspaceRecord> = None;
    let mut longest_root_len = 0;

    for record in records {
        if !record.canonical_root.exists() || !record.canonical_root.join(".git").exists() {
            continue;
        }
        let record_root = record
            .canonical_root
            .canonicalize()
            .unwrap_or_else(|_| record.canonical_root.clone());
        if canonical.starts_with(&record_root) {
            let root_len = record_root.components().count();
            if root_len > longest_root_len {
                longest_root_len = root_len;
                best_match = Some(record);
            }
        }
    }

    Ok(best_match)
}

pub fn find_workspace_record_for_path(path: &Path) -> Result<Option<WorkspaceRecord>> {
    find_workspace_record_for_path_at(path, &get_workspace_storage_dir())
}

pub fn forget_workspace_record(selector: &str) -> Result<bool> {
    let dir = get_workspace_storage_dir();
    let records = list_workspace_records()?;
    let mut removed = false;

    for record in records {
        let root_str = record.canonical_root.to_string_lossy();
        if record.workspace_id == selector || root_str == selector {
            let file_path = dir.join(format!("{}.json", record.workspace_id));
            if file_path.exists() {
                fs::remove_file(file_path)?;
                removed = true;
            }
        }
    }

    Ok(removed)
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

pub struct RepositoryLock {
    pub(crate) path: PathBuf,
}

impl RepositoryLock {
    pub fn acquire(repo: &GitRepo, operation_id: &str, name: &str) -> Result<Self> {
        let path = locks_dir(repo)?.join(name);
        let open = || OpenOptions::new().write(true).create_new(true).open(&path);
        let mut file = match open() {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = fs::read_to_string(&path)
                    .ok()
                    .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok());
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
                if live {
                    return Err(StaircaseError::OperationInProgress {
                        operation_id: existing
                            .as_ref()
                            .and_then(|lock| lock.get("operation_id"))
                            .and_then(|id| id.as_str())
                            .unwrap_or(operation_id)
                            .into(),
                        kind: "repository-lock".into(),
                    });
                }
                fs::remove_file(&path)?;
                open()?
            }
            Err(error) => return Err(error.into()),
        };
        let lock = serde_json::json!({
            "schema": "git-staircase/operation-lock",
            "version": 1,
            "pid": std::process::id(),
            "operation_id": operation_id,
            "nonce": Uuid::new_v4().to_string(),
        });
        file.write_all(serde_json::to_string(&lock)?.as_bytes())?;
        file.sync_all()?;
        Ok(Self { path })
    }
}

pub struct OperationLocks {
    pub(crate) _repository: RepositoryLock,
    pub(crate) _lineage: Option<RepositoryLock>,
}

impl OperationLocks {
    pub fn acquire(repo: &GitRepo, operation_id: &str, lineage_id: Option<&str>) -> Result<Self> {
        let repository = RepositoryLock::acquire(repo, operation_id, "repository.lock")?;
        let lineage = lineage_id
            .map(|lineage_id| {
                RepositoryLock::acquire(repo, operation_id, &format!("lineage-{}.lock", lineage_id))
            })
            .transpose()?;
        Ok(Self {
            _repository: repository,
            _lineage: lineage,
        })
    }
}

impl Drop for RepositoryLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn locks_dir(repo: &GitRepo) -> Result<PathBuf> {
    let path = staircase_dir(repo)?.join("locks");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn staircase_dir(repo: &GitRepo) -> Result<PathBuf> {
    let path = repo.common_dir()?.join("staircase");
    fs::create_dir_all(&path)?;
    Ok(path)
}

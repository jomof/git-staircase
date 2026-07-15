use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRepoEntry {
    pub relative_path: PathBuf,
    pub worktree_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonorepoWorktreeEntry {
    pub id: String,
    pub path: PathBuf,
    pub primary_root: PathBuf,
    pub active_repos: Vec<ActiveRepoEntry>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonorepoWorktreeRegistry {
    pub worktrees: Vec<MonorepoWorktreeEntry>,
}

pub fn get_registry_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("GIT_MONOREPO_REGISTRY_PATH") {
        return PathBuf::from(override_path);
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("git-monorepo").join("worktrees.json")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("git-monorepo")
            .join("worktrees.json")
    } else {
        std::env::temp_dir()
            .join("git-monorepo")
            .join("worktrees.json")
    }
}

pub fn load_registry() -> Result<MonorepoWorktreeRegistry> {
    let path = get_registry_path();
    if !path.exists() {
        return Ok(MonorepoWorktreeRegistry::default());
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read registry file: {}", path.display()))?;
    let registry: MonorepoWorktreeRegistry = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse registry file: {}", path.display()))?;
    Ok(registry)
}

pub fn save_registry(registry: &MonorepoWorktreeRegistry) -> Result<()> {
    let path = get_registry_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(registry)?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn add_worktree_entry(entry: MonorepoWorktreeEntry) -> Result<()> {
    let mut reg = load_registry()?;
    reg.worktrees.retain(|w| w.id != entry.id && w.path != entry.path);
    reg.worktrees.push(entry);
    save_registry(&reg)
}

pub fn remove_worktree_entry(id_or_path: &str) -> Result<Option<MonorepoWorktreeEntry>> {
    let mut reg = load_registry()?;
    let canonical_target = Path::new(id_or_path).canonicalize().ok();
    let index = reg.worktrees.iter().position(|w| {
        w.id == id_or_path
            || w.path.to_string_lossy() == id_or_path
            || canonical_target.as_ref().is_some_and(|target| &w.path == target)
    });
    if let Some(idx) = index {
        let removed = reg.worktrees.remove(idx);
        save_registry(&reg)?;
        Ok(Some(removed))
    } else {
        Ok(None)
    }
}

pub fn find_shadow_worktree_for_path(path: &Path) -> Result<Option<MonorepoWorktreeEntry>> {
    let reg = load_registry()?;
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    for entry in reg.worktrees {
        let entry_canonical = entry.path.canonicalize().unwrap_or_else(|_| entry.path.clone());
        if canonical.starts_with(&entry_canonical) {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

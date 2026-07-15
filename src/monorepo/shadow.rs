use crate::git::GitRepo;
use crate::monorepo::registry::{
    ActiveRepoEntry, MonorepoWorktreeEntry, add_worktree_entry, load_registry,
    remove_worktree_entry,
};
use crate::workspace::repo_provider::{list_repo_workspace_projects, probe_repo_workspace};
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

#[derive(Debug, Clone, Default)]
pub struct CreateWorktreeOptions {
    pub repo_paths: Vec<PathBuf>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub upstream: Option<String>,
    pub base: Option<String>,
    pub name: Option<String>,
    pub custom_target_path: Option<PathBuf>,
}

pub fn get_worktrees_storage_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("GIT_MONOREPO_WORKTREES_DIR") {
        return PathBuf::from(override_dir);
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("git-monorepo").join("worktrees")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("git-monorepo")
            .join("worktrees")
    } else {
        std::env::temp_dir().join("git-monorepo").join("worktrees")
    }
}

pub fn find_primary_monorepo_root(repo: &GitRepo) -> PathBuf {
    if let Ok(Some(cand)) = probe_repo_workspace(repo) {
        return cand.workspace_root;
    }
    let mut current = repo
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone());
    loop {
        if current.join("WORKSPACE").exists()
            || current.join("MODULE.bazel").exists()
            || current.join(".repo").is_dir()
        {
            return current;
        }
        if !current.pop() {
            break;
        }
    }
    repo.workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone())
}

pub fn create_monorepo_worktree(
    repo: &GitRepo,
    options: &CreateWorktreeOptions,
) -> Result<MonorepoWorktreeEntry> {
    let primary_root = find_primary_monorepo_root(repo);

    let id = if let Some(ref name) = options.name {
        format!("shadow-{}", name)
    } else {
        format!("shadow-{}", &uuid::Uuid::new_v4().to_string()[..8])
    };

    let target_path = if let Some(ref custom) = options.custom_target_path {
        custom.clone()
    } else {
        get_worktrees_storage_dir().join(&id)
    };

    if target_path.exists() {
        fs::remove_dir_all(&target_path)?;
    }
    fs::create_dir_all(&target_path)?;

    // Determine active repository relative paths
    let active_rel_paths: Vec<PathBuf> = if options.repo_paths.is_empty() {
        let active_dir = repo
            .workdir
            .canonicalize()
            .unwrap_or_else(|_| repo.workdir.clone());
        let rel = active_dir
            .strip_prefix(&primary_root)
            .unwrap_or(Path::new(""))
            .to_path_buf();
        vec![rel]
    } else {
        let mut paths = Vec::new();
        for input_path in &options.repo_paths {
            let abs_path = if input_path.is_absolute() {
                input_path.clone()
            } else if primary_root.join(input_path).exists() {
                primary_root.join(input_path)
            } else {
                repo.workdir.join(input_path)
            };
            let canonical_abs = abs_path.canonicalize().unwrap_or(abs_path);
            let rel = canonical_abs
                .strip_prefix(&primary_root)
                .unwrap_or(Path::new(""))
                .to_path_buf();
            paths.push(rel);
        }
        paths
    };

    // Get all known project relative paths from repo provider if available
    let known_projects = list_repo_workspace_projects(repo).unwrap_or_default();

    let mut active_entries = Vec::new();

    // Create Git worktrees for active repositories
    for active_rel in &active_rel_paths {
        let primary_repo_dir = primary_root.join(active_rel);
        let worktree_dest = if active_rel.as_os_str().is_empty()
            || active_rel == Path::new(".")
            || active_rel == Path::new("")
        {
            target_path.clone()
        } else {
            target_path.join(active_rel)
        };

        if let Some(parent) = worktree_dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Run git worktree add
        let mut cmd = Command::new("git");
        cmd.current_dir(&primary_repo_dir);
        cmd.arg("worktree").arg("add");

        if let Some(ref branch) = options.branch {
            if let Some(ref base) = options.base {
                cmd.arg("-b").arg(branch).arg(&worktree_dest).arg(base);
            } else {
                // Try creating branch first, if it fails because it exists, just checkout
                let check_branch = Command::new("git")
                    .current_dir(&primary_repo_dir)
                    .args(&[
                        "show-ref",
                        "--verify",
                        "--quiet",
                        &format!("refs/heads/{}", branch),
                    ])
                    .status();
                if check_branch.is_ok_and(|s| s.success()) {
                    cmd.arg(&worktree_dest).arg(branch);
                } else {
                    cmd.arg("-b").arg(branch).arg(&worktree_dest);
                }
            }
        } else if let Some(ref commit) = options.commit {
            cmd.arg("--detach").arg(&worktree_dest).arg(commit);
        } else {
            cmd.arg("--detach").arg(&worktree_dest).arg("HEAD");
        }

        let output = cmd.output().with_context(|| {
            format!(
                "Failed to run git worktree add for repository {}",
                primary_repo_dir.display()
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "git worktree add failed for {}: {}",
                primary_repo_dir.display(),
                stderr.trim()
            ));
        }

        if let Some(ref upstream) = options.upstream {
            if let Some(ref branch) = options.branch {
                let _ = Command::new("git")
                    .current_dir(&worktree_dest)
                    .args(&["branch", "--set-upstream-to", upstream, branch])
                    .output();
            }
        }

        // Find worktree metadata path inside .git/worktrees
        let git_dir_output = Command::new("git")
            .current_dir(&worktree_dest)
            .args(&["rev-parse", "--git-dir"])
            .output();
        let worktree_meta_path = if let Ok(out) = git_dir_output {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            PathBuf::from(path_str)
        } else {
            worktree_dest.clone()
        };

        active_entries.push(ActiveRepoEntry {
            relative_path: active_rel.clone(),
            worktree_path: worktree_meta_path,
        });
    }

    // Mirror directories and files from primary_root to target_path
    mirror_directory(
        &primary_root,
        &target_path,
        &primary_root,
        &active_rel_paths,
        &known_projects,
    )?;

    // Setup Bazel integration if Bazel workspace files exist
    if primary_root.join("WORKSPACE").exists() || primary_root.join("MODULE.bazel").exists() {
        setup_bazel_config(&primary_root, &target_path, &id)?;
    }

    let entry = MonorepoWorktreeEntry {
        id,
        path: target_path.canonicalize().unwrap_or(target_path),
        primary_root: primary_root.canonicalize().unwrap_or(primary_root),
        active_repos: active_entries,
        created_at: timestamp_iso8601(),
    };

    add_worktree_entry(entry.clone())?;

    Ok(entry)
}

fn mirror_directory(
    current_primary: &Path,
    current_shadow: &Path,
    primary_root: &Path,
    active_rel_paths: &[PathBuf],
    known_projects: &[PathBuf],
) -> Result<()> {
    if !current_primary.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(current_primary)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        let primary_path = entry.path();
        let rel_path = primary_path
            .strip_prefix(primary_root)
            .unwrap_or(&primary_path)
            .to_path_buf();

        // 1. Exclusions
        if is_excluded(&name_str, &rel_path) {
            continue;
        }

        // 2. Active Repository
        if active_rel_paths.iter().any(|p| p == &rel_path) {
            // Already created via git worktree add
            continue;
        }

        // 3. Parent of an Active Repository
        if active_rel_paths.iter().any(|p| p.starts_with(&rel_path)) {
            let next_shadow = current_shadow.join(&file_name);
            if !next_shadow.exists() {
                fs::create_dir_all(&next_shadow)?;
            }
            mirror_directory(
                &primary_path,
                &next_shadow,
                primary_root,
                active_rel_paths,
                known_projects,
            )?;
            continue;
        }

        // 4. Other File or Directory -> Symlink
        let shadow_dest = current_shadow.join(&file_name);
        if shadow_dest.exists() || fs::symlink_metadata(&shadow_dest).is_ok() {
            continue;
        }

        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&primary_path, &shadow_dest);
        }
        #[cfg(not(unix))]
        {
            if primary_path.is_dir() {
                let _ = std::os::windows::fs::symlink_dir(&primary_path, &shadow_dest);
            } else {
                let _ = std::os::windows::fs::symlink_file(&primary_path, &shadow_dest);
            }
        }
    }
    Ok(())
}

fn is_excluded(name: &str, rel_path: &Path) -> bool {
    if name == ".git"
        || name == ".repo"
        || name == ".venv"
        || name == "node_modules"
        || name == ".bazelrc"
    {
        return true;
    }
    if name == "bazel-bin"
        || name == "bazel-out"
        || name == "bazel-testlogs"
        || name.starts_with("bazel-")
    {
        return true;
    }
    let _ = rel_path;
    false
}

fn setup_bazel_config(primary_root: &Path, shadow_root: &Path, id: &str) -> Result<()> {
    let bazelrc_path = shadow_root.join(".bazelrc");
    if bazelrc_path.exists() || fs::symlink_metadata(&bazelrc_path).is_ok() {
        let _ = fs::remove_file(&bazelrc_path);
    }
    let cache_dir = get_worktrees_storage_dir()
        .parent()
        .unwrap_or(shadow_root)
        .join("cache");
    let output_base = cache_dir.join("bazel").join(id);
    let repo_cache = cache_dir.join("bazel-repo-cache");

    let mut content = String::new();
    let primary_bazelrc = primary_root.join(".bazelrc");
    if primary_bazelrc.exists() {
        content.push_str(&format!(
            "# Import primary workspace settings\ntry-import {}\n\n",
            primary_bazelrc.display()
        ));
    }
    content.push_str(&format!(
        "# Isolate output base\nstartup --output_base={}\n\n",
        output_base.display()
    ));
    content.push_str(&format!(
        "# Share repository cache\ncommon --repository_cache={}\n",
        repo_cache.display()
    ));

    fs::write(bazelrc_path, content)?;
    Ok(())
}

pub fn remove_monorepo_worktree(id_or_path: &str, _force: bool) -> Result<bool> {
    let removed_entry = remove_worktree_entry(id_or_path)?;
    let Some(entry) = removed_entry else {
        return Ok(false);
    };

    for active in &entry.active_repos {
        let primary_repo = entry.primary_root.join(&active.relative_path);
        if primary_repo.exists() {
            let _ = Command::new("git")
                .current_dir(&primary_repo)
                .args(&[
                    "worktree",
                    "remove",
                    "--force",
                    &active.worktree_path.to_string_lossy(),
                ])
                .output();
            let _ = Command::new("git")
                .current_dir(&primary_repo)
                .args(&["worktree", "prune"])
                .output();
        }
    }

    if entry.path.exists() {
        let _ = fs::remove_dir_all(&entry.path);
    }

    Ok(true)
}

pub fn prune_monorepo_worktrees(force: bool, _age_limit_secs: Option<u64>) -> Result<Vec<String>> {
    let reg = load_registry()?;
    let mut pruned_ids = Vec::new();

    for entry in reg.worktrees {
        let primary_missing = !entry.primary_root.exists();
        let shadow_missing = !entry.path.exists();

        if primary_missing || shadow_missing || force {
            let id = entry.id.clone();
            if remove_monorepo_worktree(&id, true)? {
                pruned_ids.push(id);
            }
        }
    }

    Ok(pruned_ids)
}

fn timestamp_iso8601() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple ISO 8601 UTC formatting: YYYY-MM-DDTHH:MM:SSZ
    let days_since_epoch = now / 86400;
    let seconds_into_day = now % 86400;
    let hours = seconds_into_day / 3600;
    let minutes = (seconds_into_day % 3600) / 60;
    let seconds = seconds_into_day % 60;

    // Approximate year, month, day calculation for RFC 3339 string
    let mut days = days_since_epoch;
    let mut year = 1970;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &length in &month_lengths {
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }
    let day = days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

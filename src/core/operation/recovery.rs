use crate::error::Result;
use crate::git::GitRepo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftRecovery {
    pub head_oid: String,
    pub head_ref: Option<String>,
    pub index_tree_oid: Option<String>,
    pub index_snapshot: Option<String>,
    #[serde(default)]
    pub dirty_files: Vec<crate::model::DraftFileSnapshot>,
    pub unstaged_patch: String,
    pub untracked_paths: Vec<String>,
    #[serde(default)]
    pub untracked_files: Vec<crate::model::DraftFileSnapshot>,
    pub attachment_json: Option<String>,
}

pub fn capture_draft(repo: &GitRepo) -> Result<Option<DraftRecovery>> {
    let head_oid = match repo.resolve_commit_opt("HEAD")? {
        Some(oid) => oid,
        None => return Ok(None),
    };
    let (index_tree_oid, index_snapshot, dirty_files) = match repo.run(&["write-tree"]) {
        Ok(oid) => (Some(oid.trim().to_string()), None, Vec::new()),
        Err(_) => {
            let (snapshot, files) = capture_conflicted_state(repo)?;
            (None, snapshot, files)
        }
    };
    let unstaged_patch = repo
        .command()
        .args(["diff", "--binary", "--no-ext-diff"])
        .trim(false)
        .run()
        .unwrap_or_default();
    let untracked = repo
        .command()
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .trim(false)
        .run()
        .unwrap_or_default();
    let attachment_path = repo.git_dir()?.join("staircase-draft.json");
    let attachment_json = fs::read_to_string(attachment_path).ok();
    Ok(Some(DraftRecovery {
        head_oid,
        head_ref: repo.current_branch()?,
        index_tree_oid,
        index_snapshot,
        dirty_files,
        unstaged_patch,
        untracked_paths: untracked
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect(),
        untracked_files: crate::core::draft::capture_untracked_files(repo)?,
        attachment_json,
    }))
}

pub fn restore_draft(repo: &GitRepo, draft: Option<&DraftRecovery>) -> Result<bool> {
    let Some(draft) = draft else { return Ok(false) };
    if let Some(head_ref) = &draft.head_ref {
        let branch = head_ref.strip_prefix("refs/heads/").unwrap_or(head_ref);
        repo.run(&["checkout", "-f", branch])?;
    } else {
        repo.run(&["checkout", "--detach", "-f", &draft.head_oid])?;
    }
    if let Some(tree) = &draft.index_tree_oid {
        repo.run(&["read-tree", tree])?;
        repo.run(&["checkout-index", "-a", "-f"])?;
    } else if let Some(snapshot) = &draft.index_snapshot {
        repo.run(&["read-tree", "--empty"])?;
        repo.run_with_stdin(&["update-index", "--index-info"], snapshot)?;
    }
    if !draft.dirty_files.is_empty() {
        restore_dirty_files(repo, &draft.dirty_files)?;
    }
    if !draft.unstaged_patch.is_empty() {
        let _ = repo.run_with_stdin(&["apply", "--binary"], &draft.unstaged_patch);
    }
    crate::core::draft::restore_untracked_files(repo, &draft.untracked_files)?;
    let attachment_path = repo.git_dir()?.join("staircase-draft.json");
    match &draft.attachment_json {
        Some(content) => fs::write(attachment_path, content)?,
        None if attachment_path.exists() => fs::remove_file(attachment_path)?,
        None => {}
    }
    Ok(true)
}

pub fn restore_draft_after_success(repo: &GitRepo, draft: Option<&DraftRecovery>) -> Result<bool> {
    let Some(draft) = draft else { return Ok(false) };
    let clean_index = draft.index_tree_oid.as_ref().is_some_and(|tree| {
        repo.get_tree_id(&draft.head_oid).ok().as_deref() == Some(tree.as_str())
    });
    if clean_index
        && draft.unstaged_patch.is_empty()
        && draft.untracked_files.is_empty()
        && draft.attachment_json.is_none()
    {
        if let Some(head_ref) = &draft.head_ref {
            let branch = head_ref.strip_prefix("refs/heads/").unwrap_or(head_ref);
            repo.run(&["checkout", "-f", branch])?;
        } else {
            repo.run(&["checkout", "--detach", "-f", &draft.head_oid])?;
        }
        return Ok(true);
    }
    restore_draft(repo, Some(draft))
}

fn capture_conflicted_state(
    repo: &GitRepo,
) -> Result<(Option<String>, Vec<crate::model::DraftFileSnapshot>)> {
    let index_snapshot = repo.run(&["ls-files", "-s"]).ok();
    let mut paths = std::collections::BTreeSet::new();
    let modified = repo
        .command()
        .args(["ls-files", "-m", "-z"])
        .trim(false)
        .run()
        .unwrap_or_default();
    for path in modified.split('\0').filter(|p| !p.is_empty()) {
        paths.insert(path.to_string());
    }
    let unmerged = repo
        .command()
        .args(["ls-files", "-u", "-z"])
        .trim(false)
        .run()
        .unwrap_or_default();
    for line in unmerged.split('\0').filter(|p| !p.is_empty()) {
        if let Some(path) = line.split('\t').last() {
            paths.insert(path.to_string());
        }
    }
    let mut files = Vec::new();
    for path in paths {
        let absolute = repo.workdir.join(&path);
        if !absolute.exists() {
            continue;
        }
        let metadata = fs::symlink_metadata(&absolute)?;
        let (kind, content) = if metadata.file_type().is_symlink() {
            (
                "symlink".to_string(),
                fs::read_link(&absolute)?.into_os_string().into_vec(),
            )
        } else if metadata.is_file() {
            ("regular".to_string(), fs::read(&absolute)?)
        } else {
            continue;
        };
        files.push(crate::model::DraftFileSnapshot {
            path: path.to_string(),
            kind,
            mode: metadata.permissions().mode(),
            content_hex: crate::core::draft::hex_encode(&content),
        });
    }
    Ok((index_snapshot, files))
}

fn restore_dirty_files(repo: &GitRepo, files: &[crate::model::DraftFileSnapshot]) -> Result<()> {
    for file in files {
        let absolute = repo.workdir.join(&file.path);
        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = crate::core::draft::hex_decode(&file.content_hex)?;
        match file.kind.as_str() {
            "regular" => {
                fs::write(&absolute, content)?;
                fs::set_permissions(&absolute, fs::Permissions::from_mode(file.mode))?;
            }
            "symlink" => {
                let _ = fs::remove_file(&absolute);
                std::os::unix::fs::symlink(std::ffi::OsString::from_vec(content), &absolute)?;
            }
            _ => {}
        }
    }
    Ok(())
}

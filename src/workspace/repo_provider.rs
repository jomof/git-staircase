use crate::error::Result;
use crate::git::GitRepo;
use crate::workspace::model::{
    Capability, ProbeDescriptor, ProjectInfo, ProviderDescriptor, WorkspaceCandidate,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_repo_descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        protocol_version: 1,
        name: "repo".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            Capability::Workspace,
            Capability::ProjectMapping,
            Capability::IntegrationContext,
            Capability::Review,
            Capability::Verification,
            Capability::Transport,
        ],
        probe: ProbeDescriptor {
            passive: true,
            network: false,
            mutates_workspace: false,
        },
    }
}

#[derive(Debug, Clone)]
pub struct RepoManifestInfo {
    pub client_root: PathBuf,
    pub project_name: String,
    pub project_path: PathBuf,
    pub revision: String,
    pub upstream: Option<String>,
    pub dest_branch: Option<String>,
    pub remote_name: Option<String>,
    pub review_endpoint: Option<String>,
}

pub fn probe_repo_workspace(repo: &GitRepo) -> Result<Option<WorkspaceCandidate>> {
    let canonical_workdir = repo
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone());

    let repo_client_root = match find_repo_client_root(&canonical_workdir) {
        Some(root) => root,
        None => return Ok(None),
    };

    // Make sure we aren't inside internal .repo git objects directory
    let repo_meta_dir = repo_client_root.join(".repo");
    if canonical_workdir.starts_with(&repo_meta_dir) {
        return Ok(None);
    }

    let manifest_info = match parse_repo_manifest_for_project(&repo_client_root, &canonical_workdir) {
        Ok(Some(info)) => info,
        _ => return Ok(None),
    };

    let project_rel_path = manifest_info
        .project_path
        .strip_prefix(&repo_client_root)
        .unwrap_or(&manifest_info.project_path)
        .to_path_buf();

    let mut fingerprint = HashMap::new();
    fingerprint.insert("provider".to_string(), "repo".to_string());
    fingerprint.insert(
        "workspace_root".to_string(),
        repo_client_root.display().to_string(),
    );
    fingerprint.insert("project_name".to_string(), manifest_info.project_name.clone());
    fingerprint.insert("project_path".to_string(), project_rel_path.display().to_string());
    fingerprint.insert("revision".to_string(), manifest_info.revision.clone());
    if let Some(ref u) = manifest_info.upstream {
        fingerprint.insert("upstream".to_string(), u.clone());
    }
    if let Some(ref db) = manifest_info.dest_branch {
        fingerprint.insert("dest_branch".to_string(), db.clone());
    }
    if let Some(ref r) = manifest_info.review_endpoint {
        fingerprint.insert("review_endpoint".to_string(), r.clone());
    }

    let candidate = WorkspaceCandidate {
        provider: "repo".to_string(),
        workspace_root: repo_client_root.clone(),
        workspace_key: Some(format!("repo:{}", repo_client_root.display())),
        current_project: Some(ProjectInfo {
            path: project_rel_path,
            identity: manifest_info.project_name.clone(),
        }),
        claim: "authoritative".to_string(),
        confidence: "high".to_string(),
        evidence: vec![
            "valid repo client".to_string(),
            "current Git repository matches one manifest project".to_string(),
        ],
        fingerprint,
    };

    Ok(Some(candidate))
}

fn find_repo_client_root(dir: &Path) -> Option<PathBuf> {
    let mut current = dir.to_path_buf();
    loop {
        let dot_repo = current.join(".repo");
        if dot_repo.is_dir() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn parse_repo_manifest_for_project(
    client_root: &Path,
    workdir: &Path,
) -> Result<Option<RepoManifestInfo>> {
    let dot_repo = client_root.join(".repo");
    let manifest_file = dot_repo.join("manifest.xml");

    if !manifest_file.exists() {
        return Ok(None);
    }

    let mut remotes: HashMap<String, RepoRemote> = HashMap::new();
    let mut default_config = RepoDefault::default();
    let mut projects: Vec<RepoProject> = Vec::new();

    parse_manifest_xml_file(&manifest_file, &dot_repo, &mut remotes, &mut default_config, &mut projects)?;

    // Check local_manifests if present
    let local_manifests_dir = dot_repo.join("local_manifests");
    if local_manifests_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(local_manifests_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) == Some("xml") {
                    let _ = parse_manifest_xml_file(&p, &dot_repo, &mut remotes, &mut default_config, &mut projects);
                }
            }
        }
    }
    let local_manifest_file = dot_repo.join("local_manifest.xml");
    if local_manifest_file.is_file() {
        let _ = parse_manifest_xml_file(&local_manifest_file, &dot_repo, &mut remotes, &mut default_config, &mut projects);
    }

    // Find project matching workdir
    for proj in projects {
        let proj_path = client_root.join(if proj.path.is_empty() { &proj.name } else { &proj.path });
        let canonical_proj_path = proj_path.canonicalize().unwrap_or(proj_path.clone());

        if workdir == canonical_proj_path || workdir.starts_with(&canonical_proj_path) {
            let revision = proj
                .revision
                .or_else(|| default_config.revision.clone())
                .unwrap_or_else(|| "main".to_string());
            let remote_name = proj.remote.or_else(|| default_config.remote.clone());
            let dest_branch = proj.dest_branch.or_else(|| default_config.dest_branch.clone());
            let review_endpoint = remote_name
                .as_ref()
                .and_then(|r| remotes.get(r))
                .and_then(|r| r.review.clone());

            return Ok(Some(RepoManifestInfo {
                client_root: client_root.to_path_buf(),
                project_name: proj.name,
                project_path: canonical_proj_path,
                revision,
                upstream: proj.upstream,
                dest_branch,
                remote_name,
                review_endpoint,
            }));
        }
    }

    Ok(None)
}

#[derive(Debug, Clone, Default)]
struct RepoRemote {
    name: String,
    #[allow(dead_code)]
    fetch: Option<String>,
    review: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct RepoDefault {
    revision: Option<String>,
    remote: Option<String>,
    dest_branch: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct RepoProject {
    name: String,
    path: String,
    revision: Option<String>,
    remote: Option<String>,
    dest_branch: Option<String>,
    upstream: Option<String>,
}

fn parse_manifest_xml_file(
    xml_path: &Path,
    dot_repo: &Path,
    remotes: &mut HashMap<String, RepoRemote>,
    default_config: &mut RepoDefault,
    projects: &mut Vec<RepoProject>,
) -> Result<()> {
    let content = match fs::read_to_string(xml_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("<remote ") || line.contains("<remote ") {
            if let Some(r) = parse_remote_tag(line) {
                remotes.insert(r.name.clone(), r);
            }
        } else if line.starts_with("<default ") || line.contains("<default ") {
            parse_default_tag(line, default_config);
        } else if line.starts_with("<project ") || line.contains("<project ") {
            if let Some(p) = parse_project_tag(line) {
                projects.push(p);
            }
        } else if line.starts_with("<include ") || line.contains("<include ") {
            if let Some(inc_name) = parse_attr(line, "name") {
                let inc_path = dot_repo.join("manifests").join(&inc_name);
                if inc_path.exists() {
                    let _ = parse_manifest_xml_file(&inc_path, dot_repo, remotes, default_config, projects);
                }
            }
        }
    }

    Ok(())
}

fn parse_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(pos) = tag.find(&pattern) {
        let start = pos + pattern.len();
        if let Some(end) = tag[start..].find('"') {
            return Some(tag[start..start + end].to_string());
        }
    }
    let pattern_single = format!("{}='", attr);
    if let Some(pos) = tag.find(&pattern_single) {
        let start = pos + pattern_single.len();
        if let Some(end) = tag[start..].find('\'') {
            return Some(tag[start..start + end].to_string());
        }
    }
    None
}

fn parse_remote_tag(tag: &str) -> Option<RepoRemote> {
    let name = parse_attr(tag, "name")?;
    let fetch = parse_attr(tag, "fetch");
    let review = parse_attr(tag, "review");
    Some(RepoRemote { name, fetch, review })
}

fn parse_default_tag(tag: &str, default_config: &mut RepoDefault) {
    if let Some(rev) = parse_attr(tag, "revision") {
        default_config.revision = Some(rev);
    }
    if let Some(rem) = parse_attr(tag, "remote") {
        default_config.remote = Some(rem);
    }
    if let Some(db) = parse_attr(tag, "dest-branch") {
        default_config.dest_branch = Some(db);
    }
}

fn parse_project_tag(tag: &str) -> Option<RepoProject> {
    let name = parse_attr(tag, "name")?;
    let path = parse_attr(tag, "path").unwrap_or_else(|| name.clone());
    let revision = parse_attr(tag, "revision");
    let remote = parse_attr(tag, "remote");
    let dest_branch = parse_attr(tag, "dest-branch");
    let upstream = parse_attr(tag, "upstream");

    Some(RepoProject {
        name,
        path,
        revision,
        remote,
        dest_branch,
        upstream,
    })
}

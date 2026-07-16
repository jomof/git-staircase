use crate::error::Result;
use crate::git::GitRepo;
use crate::workspace::model::{
    Capability, EvidenceAuthority, ProbeDescriptor, ProjectInfo, ProviderDescriptor, TypedEvidence,
    WorkspaceCandidate, WorkspaceHint, WorkspaceRecord,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub fn get_repo_descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        protocol_version: 1,
        name: "repo".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            Capability::Workspace,
            Capability::ProjectMapping,
            Capability::IntegrationContext,
            Capability::WorkspaceHints,
        ],
        probe: ProbeDescriptor {
            passive: true,
            network: false,
            authenticates: false,
            mutates_workspace: false,
            executes_repository_hooks: false,
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
    pub fetch_url: Option<String>,
    pub git_common_dir: PathBuf,
    pub translated_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCheckoutEvidence {
    pub oid: Option<String>,
    pub head_state: String,
    pub branch: Option<String>,
    pub source: String,
    pub active_git_operation: Option<String>,
    pub relation_to_manifest: Option<String>,
    pub eligible_as_anchor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoProjectMapping {
    pub workspace_root: PathBuf,
    pub project_name: String,
    pub project_path: PathBuf,
    pub git_common_dir: PathBuf,
    pub checkout_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoDiscoveryReport {
    pub candidate: WorkspaceCandidate,
    pub mapping: RepoProjectMapping,
    pub checkout: RepoCheckoutEvidence,
    pub integration_candidates: Vec<TypedEvidence>,
    pub hints: Vec<WorkspaceHint>,
    pub diagnostics: Vec<String>,
    pub executable_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRefresh {
    pub changed: bool,
    pub previous_fingerprint: HashMap<String, String>,
    pub current: RepoDiscoveryReport,
    pub observational_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoForallInvocation {
    pub executable: String,
    pub arguments: Vec<String>,
    pub fixed_body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoForallMetadata {
    pub project: String,
    pub path: String,
    pub remote: Option<String>,
    pub translated_revision: Option<String>,
    pub manifest_revision: String,
    pub upstream: Option<String>,
    pub destination_branch: Option<String>,
    pub fetch_url: Option<String>,
}

pub trait RepoMetadataSource: Send + Sync {
    fn current_project(
        &self,
        client_root: &Path,
        worktree: &Path,
    ) -> Result<Option<RepoForallMetadata>>;
}

#[derive(Default)]
pub struct ProductionRepoMetadataSource;

impl RepoMetadataSource for ProductionRepoMetadataSource {
    fn current_project(
        &self,
        _client_root: &Path,
        worktree: &Path,
    ) -> Result<Option<RepoForallMetadata>> {
        let invocation = controlled_repo_forall_invocation();
        let mut child = match Command::new(&invocation.executable)
            .current_dir(worktree)
            .args(&invocation.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("GIT_TERMINAL_PROMPT", "0")
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let started = Instant::now();
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if started.elapsed() >= Duration::from_secs(2) {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output()?;
        if !output.status.success() || output.stdout.len() > 64 * 1024 {
            return Ok(None);
        }
        parse_repo_forall_output(&output.stdout)
    }
}

pub fn controlled_repo_forall_invocation() -> RepoForallInvocation {
    const BODY: &str = r#"printf '%s\0%s\0%s\0%s\0%s\0%s\0%s\0%s\0' "$REPO_PROJECT" "$REPO_PATH" "$REPO_REMOTE" "$REPO_LREV" "$REPO_RREV" "$REPO_UPSTREAM" "$REPO_DEST_BRANCH" "$REPO_PROJECT_FETCH_URL""#;
    RepoForallInvocation {
        executable: "repo".into(),
        arguments: vec!["forall".into(), ".".into(), "-c".into(), BODY.into()],
        fixed_body: BODY.into(),
    }
}

pub fn parse_repo_forall_output(output: &[u8]) -> Result<Option<RepoForallMetadata>> {
    let mut fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    if fields.last().is_some_and(|field| field.is_empty()) {
        fields.pop();
    }
    if fields.is_empty() {
        return Ok(None);
    }
    if fields.len() != 8 {
        return Err(crate::error::StaircaseError::Other(
            "malformed controlled repo forall metadata".into(),
        ));
    }
    let values = fields
        .into_iter()
        .map(|field| {
            std::str::from_utf8(field).map(str::to_string).map_err(|_| {
                crate::error::StaircaseError::Other(
                    "repo forall metadata is not valid UTF-8".into(),
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if values[0].is_empty() || values[1].is_empty() || values[4].is_empty() {
        return Err(crate::error::StaircaseError::Other(
            "repo forall metadata is incomplete".into(),
        ));
    }
    let optional = |value: &String| (!value.is_empty()).then(|| value.clone());
    Ok(Some(RepoForallMetadata {
        project: values[0].clone(),
        path: values[1].clone(),
        remote: optional(&values[2]),
        translated_revision: optional(&values[3]),
        manifest_revision: values[4].clone(),
        upstream: optional(&values[5]),
        destination_branch: optional(&values[6]),
        fetch_url: optional(&values[7]),
    }))
}

pub fn probe_repo_workspace(repo: &GitRepo) -> Result<Option<WorkspaceCandidate>> {
    probe_repo_workspace_with_source(repo, &ProductionRepoMetadataSource)
}

pub fn probe_repo_workspace_with_source(
    repo: &GitRepo,
    source: &dyn RepoMetadataSource,
) -> Result<Option<WorkspaceCandidate>> {
    let canonical_workdir = repo
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| repo.workdir.clone());

    let repo_client_root = match find_repo_client_root(&canonical_workdir).or_else(|| {
        git_common_dir_for(&canonical_workdir)
            .as_deref()
            .and_then(find_repo_client_root)
    }) {
        Some(root) => root,
        None => return Ok(None),
    };

    // Make sure we aren't inside internal .repo git objects directory
    let repo_meta_dir = repo_client_root.join(".repo");
    if canonical_workdir.starts_with(&repo_meta_dir) {
        return Ok(None);
    }

    let manifest_info =
        match parse_repo_manifest_for_project(&repo_client_root, &canonical_workdir)? {
            Some(info) => info,
            None => match source.current_project(&repo_client_root, &canonical_workdir)? {
                Some(metadata) => {
                    repo_forall_manifest_info(&repo_client_root, &canonical_workdir, metadata)?
                }
                None => return Ok(None),
            },
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
    fingerprint.insert(
        "project_name".to_string(),
        manifest_info.project_name.clone(),
    );
    fingerprint.insert(
        "project_path".to_string(),
        project_rel_path.display().to_string(),
    );
    fingerprint.insert("revision".to_string(), manifest_info.revision.clone());
    fingerprint.insert(
        "git_common_dir".to_string(),
        manifest_info.git_common_dir.display().to_string(),
    );
    fingerprint.insert(
        "effective_manifest".to_string(),
        effective_manifest_fingerprint(&repo_client_root),
    );
    if let Ok(head) = repo.resolve_commit("HEAD") {
        fingerprint.insert("checkout_head".into(), head);
    }
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

pub fn observe_repo_workspace(repo: &GitRepo) -> Result<Option<RepoDiscoveryReport>> {
    observe_repo_workspace_with_source(repo, &ProductionRepoMetadataSource)
}

pub fn observe_repo_workspace_with_source(
    repo: &GitRepo,
    source: &dyn RepoMetadataSource,
) -> Result<Option<RepoDiscoveryReport>> {
    let Some(candidate) = probe_repo_workspace_with_source(repo, source)? else {
        return Ok(None);
    };
    let client_root = candidate.workspace_root.clone();
    let worktree_root = git_worktree_root_for(&repo.workdir).unwrap_or_else(|| {
        repo.workdir
            .canonicalize()
            .unwrap_or_else(|_| repo.workdir.clone())
    });
    let info = match parse_repo_manifest_for_project(&client_root, &worktree_root)? {
        Some(info) => info,
        None => match source.current_project(&client_root, &worktree_root)? {
            Some(metadata) => repo_forall_manifest_info(&client_root, &worktree_root, metadata)?,
            None => return Ok(None),
        },
    };
    let project_path = info
        .project_path
        .strip_prefix(&client_root)
        .unwrap_or(&info.project_path)
        .to_path_buf();
    let checkout_identity = checkout_identity(
        &client_root,
        &info.project_name,
        &project_path,
        &info.git_common_dir,
    );
    let head_oid = repo.resolve_commit("HEAD").ok();
    let branch = repo
        .run(&["symbolic-ref", "--quiet", "--short", "HEAD"])
        .ok()
        .filter(|value| !value.is_empty());
    let active_git_operation = active_git_operation(repo, &info.git_common_dir);
    let manifest_resolved = resolve_manifest_locator(repo, &info.revision);
    let relation = match (&head_oid, &manifest_resolved) {
        (Some(head), Some(manifest)) if head == manifest => Some("equal".into()),
        (Some(head), Some(manifest)) if repo.is_ancestor(manifest, head).unwrap_or(false) => {
            Some("ahead".into())
        }
        (Some(head), Some(manifest)) if repo.is_ancestor(head, manifest).unwrap_or(false) => {
            Some("behind".into())
        }
        (Some(_), Some(_)) => Some("diverged".into()),
        _ => None,
    };
    let detached = branch.is_none();
    let eligible_checkout = detached
        && active_git_operation.is_none()
        && relation
            .as_deref()
            .is_some_and(|relation| matches!(relation, "equal" | "behind"));
    let checkout = RepoCheckoutEvidence {
        oid: head_oid.clone(),
        head_state: if branch.is_some() {
            "attached".into()
        } else if head_oid.is_some() {
            "detached".into()
        } else {
            "unborn".into()
        },
        branch,
        source: "project-head".into(),
        active_git_operation,
        relation_to_manifest: relation,
        eligible_as_anchor: eligible_checkout,
    };

    let mut integration_candidates = Vec::new();
    let exact_manifest = is_full_hex_oid(&info.revision);
    integration_candidates.push(TypedEvidence {
        kind: if exact_manifest {
            "exact-manifest-oid".into()
        } else {
            "declared-manifest-revision".into()
        },
        locator: Some(info.revision.clone()),
        resolved_oid: manifest_resolved.clone(),
        source: "effective-manifest".into(),
        exact: exact_manifest,
        moving: !exact_manifest,
        authority: if exact_manifest {
            EvidenceAuthority::Authoritative
        } else {
            EvidenceAuthority::Advisory
        },
        provenance: vec!["manifest project revision/default inheritance".into()],
        eligible: manifest_resolved.is_some(),
        notes: Vec::new(),
    });
    if let Some(translated) = &info.translated_revision {
        integration_candidates.push(TypedEvidence {
            kind: "translated-manifest-revision".into(),
            locator: Some(translated.clone()),
            resolved_oid: resolve_manifest_locator(repo, translated),
            source: "REPO_LREV".into(),
            exact: is_full_hex_oid(translated),
            moving: !is_full_hex_oid(translated),
            authority: EvidenceAuthority::Strong,
            provenance: vec!["controlled repo forall metadata".into()],
            eligible: true,
            notes: Vec::new(),
        });
    }
    if detached {
        integration_candidates.push(TypedEvidence {
            kind: "detached-checkout".into(),
            locator: None,
            resolved_oid: head_oid,
            source: "project-head".into(),
            exact: true,
            moving: false,
            authority: if eligible_checkout {
                EvidenceAuthority::Strong
            } else {
                EvidenceAuthority::Ineligible
            },
            provenance: vec!["exact current checkout".into()],
            eligible: eligible_checkout,
            notes: if eligible_checkout {
                Vec::new()
            } else {
                vec!["checkout is local, transient, or incompatible with manifest evidence".into()]
            },
        });
    }
    if let Some(upstream) = &info.upstream {
        integration_candidates.push(TypedEvidence {
            kind: "upstream".into(),
            locator: Some(upstream.clone()),
            resolved_oid: resolve_manifest_locator(repo, upstream),
            source: "effective-project-upstream".into(),
            exact: is_full_hex_oid(upstream),
            moving: !is_full_hex_oid(upstream),
            authority: EvidenceAuthority::Advisory,
            provenance: vec!["manifest upstream".into()],
            eligible: true,
            notes: vec!["upstream remains separate from manifest and checkout evidence".into()],
        });
    }

    let mut hints = Vec::new();
    let freshness = candidate.fingerprint.clone();
    if let Some(endpoint) = &info.review_endpoint {
        hints.push(WorkspaceHint {
            kind: "review-endpoint".into(),
            provider_hint: Some("gerrit".into()),
            value: endpoint.clone(),
            source: "manifest-remote-review".into(),
            scope: HashMap::from([("project".into(), info.project_name.clone())]),
            confidence: "high".into(),
            normalized: false,
            freshness_fingerprint: freshness.clone(),
        });
    }
    hints.push(WorkspaceHint {
        kind: "review-project".into(),
        provider_hint: Some("gerrit".into()),
        value: info.project_name.clone(),
        source: "effective-project-name".into(),
        scope: HashMap::from([("path".into(), project_path.display().to_string())]),
        confidence: "high".into(),
        normalized: true,
        freshness_fingerprint: freshness.clone(),
    });
    if let Some(destination) = info
        .dest_branch
        .as_deref()
        .and_then(normalize_branch_destination)
        .or_else(|| {
            (!is_full_hex_oid(&info.revision))
                .then(|| normalize_branch_destination(&info.revision))
                .flatten()
        })
    {
        hints.push(WorkspaceHint {
            kind: "review-destination".into(),
            provider_hint: Some("gerrit".into()),
            value: destination,
            source: if info.dest_branch.is_some() {
                "project-dest-branch".into()
            } else {
                "manifest-branch-revision".into()
            },
            scope: HashMap::from([("project".into(), info.project_name.clone())]),
            confidence: "high".into(),
            normalized: true,
            freshness_fingerprint: freshness.clone(),
        });
    }
    if let Some(remote) = &info.remote_name {
        hints.push(WorkspaceHint {
            kind: "manifest-remote".into(),
            provider_hint: None,
            value: remote.clone(),
            source: "effective-project-remote".into(),
            scope: HashMap::from([("project".into(), info.project_name.clone())]),
            confidence: "high".into(),
            normalized: true,
            freshness_fingerprint: freshness.clone(),
        });
    }
    if let Some(fetch) = &info.fetch_url {
        hints.push(WorkspaceHint {
            kind: "fetch-url".into(),
            provider_hint: None,
            value: fetch.clone(),
            source: "manifest-remote-fetch".into(),
            scope: HashMap::from([("project".into(), info.project_name.clone())]),
            confidence: "medium".into(),
            normalized: false,
            freshness_fingerprint: freshness,
        });
    }

    Ok(Some(RepoDiscoveryReport {
        candidate,
        mapping: RepoProjectMapping {
            workspace_root: client_root,
            project_name: info.project_name,
            project_path,
            git_common_dir: info.git_common_dir,
            checkout_identity,
        },
        checkout,
        integration_candidates,
        hints,
        diagnostics: Vec::new(),
        executable_available: executable_in_path("repo"),
    }))
}

pub fn refresh_repo_workspace(
    repo: &GitRepo,
    record: &WorkspaceRecord,
) -> Result<Option<RepoRefresh>> {
    let Some(current) = observe_repo_workspace(repo)? else {
        return Ok(None);
    };
    Ok(Some(RepoRefresh {
        changed: current.candidate.fingerprint != record.discovery_fingerprint,
        previous_fingerprint: record.discovery_fingerprint.clone(),
        current,
        observational_only: true,
    }))
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

fn repo_forall_manifest_info(
    client_root: &Path,
    workdir: &Path,
    metadata: RepoForallMetadata,
) -> Result<RepoManifestInfo> {
    let relative_path = Path::new(&metadata.path);
    if relative_path.is_absolute()
        || !relative_path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(crate::error::StaircaseError::Other(
            "repo forall returned an unsafe project path".into(),
        ));
    }
    let project_path = client_root.join(relative_path);
    let canonical_project = project_path.canonicalize().unwrap_or(project_path);
    let current_root = git_worktree_root_for(workdir);
    let project_common = git_common_dir_for(&canonical_project);
    let current_common = git_common_dir_for(workdir);
    if current_root.as_ref() != Some(&canonical_project)
        && (project_common.is_none() || project_common != current_common)
    {
        return Err(crate::error::StaircaseError::Other(
            "repo forall project does not own the current Git common directory".into(),
        ));
    }
    Ok(RepoManifestInfo {
        client_root: client_root.into(),
        project_name: metadata.project,
        project_path: canonical_project.clone(),
        revision: metadata.manifest_revision,
        upstream: metadata.upstream,
        dest_branch: metadata.destination_branch,
        remote_name: metadata.remote,
        review_endpoint: None,
        fetch_url: metadata.fetch_url,
        git_common_dir: project_common.unwrap_or_else(|| canonical_project.join(".git")),
        translated_revision: metadata.translated_revision,
    })
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

    parse_manifest_xml_file(
        &manifest_file,
        &dot_repo,
        &mut remotes,
        &mut default_config,
        &mut projects,
    )?;

    // Check local_manifests if present
    let local_manifests_dir = dot_repo.join("local_manifests");
    if local_manifests_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(local_manifests_dir) {
            let mut paths = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("xml"))
                .collect::<Vec<_>>();
            paths.sort();
            for p in paths {
                if p.extension().and_then(|s| s.to_str()) == Some("xml") {
                    let _ = parse_manifest_xml_file(
                        &p,
                        &dot_repo,
                        &mut remotes,
                        &mut default_config,
                        &mut projects,
                    );
                }
            }
        }
    }
    let local_manifest_file = dot_repo.join("local_manifest.xml");
    if local_manifest_file.is_file() {
        let _ = parse_manifest_xml_file(
            &local_manifest_file,
            &dot_repo,
            &mut remotes,
            &mut default_config,
            &mut projects,
        );
    }

    let current_git_root = git_worktree_root_for(workdir);
    let current_common_dir = git_common_dir_for(workdir);
    let mut matches = Vec::new();

    // Match the exact worktree or Git common directory. Merely being nested
    // under a manifest path is not proof of project ownership.
    for proj in projects {
        let proj_path = client_root.join(if proj.path.is_empty() {
            &proj.name
        } else {
            &proj.path
        });
        let canonical_proj_path = proj_path.canonicalize().unwrap_or(proj_path.clone());

        let project_common_dir = git_common_dir_for(&canonical_proj_path);
        let path_matches = current_git_root.as_ref() == Some(&canonical_proj_path);
        let common_dir_matches = current_common_dir.is_some()
            && project_common_dir.is_some()
            && current_common_dir == project_common_dir;

        if path_matches || common_dir_matches {
            let revision = proj
                .revision
                .or_else(|| default_config.revision.clone())
                .unwrap_or_else(|| "main".to_string());
            let remote_name = proj.remote.or_else(|| default_config.remote.clone());
            let dest_branch = proj
                .dest_branch
                .or_else(|| default_config.dest_branch.clone());
            let review_endpoint = remote_name
                .as_ref()
                .and_then(|r| remotes.get(r))
                .and_then(|r| r.review.clone());
            let fetch_url = remote_name
                .as_ref()
                .and_then(|r| remotes.get(r))
                .and_then(|r| r.fetch.clone());
            let git_common_dir = git_common_dir_for(&canonical_proj_path)
                .unwrap_or_else(|| canonical_proj_path.join(".git"));

            matches.push(RepoManifestInfo {
                client_root: client_root.to_path_buf(),
                project_name: proj.name,
                project_path: canonical_proj_path,
                revision,
                upstream: proj.upstream,
                dest_branch,
                remote_name,
                review_endpoint,
                fetch_url,
                git_common_dir,
                translated_revision: None,
            });
        }
    }

    if matches.len() == 1 {
        Ok(matches.pop())
    } else if matches.len() > 1 {
        Err(crate::error::StaircaseError::Ambiguous(format!(
            "current Git common directory maps to {} effective repo projects",
            matches.len()
        )))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Default)]
struct RepoRemote {
    name: String,
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
    parse_manifest_xml_file_inner(xml_path, dot_repo, remotes, default_config, projects, 0)
}

fn parse_manifest_xml_file_inner(
    xml_path: &Path,
    dot_repo: &Path,
    remotes: &mut HashMap<String, RepoRemote>,
    default_config: &mut RepoDefault,
    projects: &mut Vec<RepoProject>,
    depth: usize,
) -> Result<()> {
    if depth > 32 {
        return Err(crate::error::StaircaseError::Other(
            "repo manifest include depth exceeded".into(),
        ));
    }
    let content = match fs::read_to_string(xml_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    for tag in xml_start_tags(&content) {
        if tag.starts_with("<remote ") {
            if let Some(r) = parse_remote_tag(&tag) {
                remotes.insert(r.name.clone(), r);
            }
        } else if tag.starts_with("<default ") {
            parse_default_tag(&tag, default_config);
        } else if tag.starts_with("<project ") {
            if let Some(p) = parse_project_tag(&tag) {
                projects.push(p);
            }
        } else if tag.starts_with("<remove-project ") {
            let name = parse_attr(&tag, "name");
            let path = parse_attr(&tag, "path");
            projects.retain(|project| {
                !((name.as_ref() == Some(&project.name))
                    && path
                        .as_ref()
                        .is_none_or(|candidate| candidate == &project.path))
            });
        } else if tag.starts_with("<extend-project ") {
            if let Some(name) = parse_attr(&tag, "name") {
                let path_selector = parse_attr(&tag, "path");
                for project in projects.iter_mut().filter(|project| {
                    project.name == name
                        && path_selector
                            .as_ref()
                            .is_none_or(|candidate| candidate == &project.path)
                }) {
                    if let Some(path) = parse_attr(&tag, "dest-path") {
                        project.path = path;
                    }
                    if let Some(revision) = parse_attr(&tag, "revision") {
                        project.revision = Some(revision);
                    }
                    if let Some(remote) = parse_attr(&tag, "remote") {
                        project.remote = Some(remote);
                    }
                    if let Some(destination) = parse_attr(&tag, "dest-branch") {
                        project.dest_branch = Some(destination);
                    }
                    if let Some(upstream) = parse_attr(&tag, "upstream") {
                        project.upstream = Some(upstream);
                    }
                }
            }
        } else if tag.starts_with("<submanifest ") {
            if let Some(name) = parse_attr(&tag, "name") {
                let prefix = parse_attr(&tag, "path").unwrap_or_else(|| name.clone());
                let manifest_name =
                    parse_attr(&tag, "manifest-name").unwrap_or_else(|| "default.xml".into());
                if safe_manifest_relative_path(&name)
                    && safe_manifest_relative_path(&prefix)
                    && safe_manifest_relative_path(&manifest_name)
                {
                    let candidates = [
                        dot_repo
                            .join("submanifests")
                            .join(&name)
                            .join("manifest.xml"),
                        dot_repo
                            .join("submanifests")
                            .join(&name)
                            .join("manifests")
                            .join(&manifest_name),
                        dot_repo.join("manifests").join(&manifest_name),
                    ];
                    if let Some(path) = candidates.into_iter().find(|path| path.exists()) {
                        let mut sub_default = default_config.clone();
                        let mut sub_projects = Vec::new();
                        let _ = parse_manifest_xml_file_inner(
                            &path,
                            dot_repo,
                            remotes,
                            &mut sub_default,
                            &mut sub_projects,
                            depth + 1,
                        );
                        for mut project in sub_projects {
                            project.path =
                                Path::new(&prefix).join(project.path).display().to_string();
                            if project.revision.is_none() {
                                project.revision = sub_default.revision.clone();
                            }
                            if project.remote.is_none() {
                                project.remote = sub_default.remote.clone();
                            }
                            if project.dest_branch.is_none() {
                                project.dest_branch = sub_default.dest_branch.clone();
                            }
                            projects.push(project);
                        }
                    }
                }
            }
        } else if tag.starts_with("<include ") {
            if let Some(inc_name) = parse_attr(&tag, "name") {
                if safe_manifest_relative_path(&inc_name) {
                    let inc_path = dot_repo.join("manifests").join(&inc_name);
                    let _ = parse_manifest_xml_file_inner(
                        &inc_path,
                        dot_repo,
                        remotes,
                        default_config,
                        projects,
                        depth + 1,
                    );
                }
            }
        }
    }

    Ok(())
}

fn xml_start_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut offset = 0;
    while let Some(start_rel) = content[offset..].find('<') {
        let start = offset + start_rel;
        let Some(end_rel) = content[start..].find('>') else {
            break;
        };
        let end = start + end_rel + 1;
        let tag = content[start..end].trim();
        if !tag.starts_with("</") && !tag.starts_with("<?") && !tag.starts_with("<!") {
            tags.push(tag.to_string());
        }
        offset = end;
    }
    tags
}

fn safe_manifest_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
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
    Some(RepoRemote {
        name,
        fetch,
        review,
    })
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

fn git_worktree_root_for(path: &Path) -> Option<PathBuf> {
    git_path_query(path, &["rev-parse", "--show-toplevel"])
}

fn git_common_dir_for(path: &Path) -> Option<PathBuf> {
    git_path_query(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
}

fn git_path_query(path: &Path, args: &[&str]) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(value.trim());
    Some(path.canonicalize().unwrap_or(path))
}

fn active_git_operation(repo: &GitRepo, common_dir: &Path) -> Option<String> {
    let git_dir = repo
        .run(&["rev-parse", "--path-format=absolute", "--git-dir"])
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| common_dir.to_path_buf());
    [
        ("rebase", common_dir.join("rebase-merge")),
        ("rebase", common_dir.join("rebase-apply")),
        ("merge", git_dir.join("MERGE_HEAD")),
        ("cherry-pick", git_dir.join("CHERRY_PICK_HEAD")),
        ("revert", git_dir.join("REVERT_HEAD")),
        ("bisect", common_dir.join("BISECT_START")),
        ("sequencer", common_dir.join("sequencer")),
    ]
    .into_iter()
    .find_map(|(name, marker)| marker.exists().then(|| name.to_string()))
}

fn resolve_manifest_locator(repo: &GitRepo, locator: &str) -> Option<String> {
    let candidates = if locator.starts_with("refs/") || is_full_hex_oid(locator) {
        vec![locator.to_string()]
    } else {
        vec![
            locator.to_string(),
            format!("refs/heads/{}", locator),
            format!("refs/remotes/{}/{}", "m", locator),
        ]
    };
    candidates
        .into_iter()
        .find_map(|candidate| repo.resolve_commit(&candidate).ok())
}

fn is_full_hex_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn normalize_branch_destination(value: &str) -> Option<String> {
    if let Some(branch) = value.strip_prefix("refs/heads/") {
        return valid_branch_component(branch).then(|| value.to_string());
    }
    if value.starts_with("refs/")
        || is_full_hex_oid(value)
        || value.contains("..")
        || value.contains(['~', '^', ':', '?', '*', '[', '\\'])
    {
        return None;
    }
    valid_branch_component(value).then(|| format!("refs/heads/{}", value))
}

fn valid_branch_component(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.ends_with('/')
        && !value.ends_with(".lock")
        && !value.contains(char::is_whitespace)
}

fn checkout_identity(root: &Path, project: &str, project_path: &Path, common_dir: &Path) -> String {
    let mut hasher = Sha256::new();
    for value in [
        root.display().to_string(),
        project.to_string(),
        project_path.display().to_string(),
        common_dir.display().to_string(),
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    format!("repo-checkout:{:x}", hasher.finalize())
}

fn effective_manifest_fingerprint(client_root: &Path) -> String {
    let dot_repo = client_root.join(".repo");
    let mut files = vec![dot_repo.join("manifest.xml")];
    for directory in [dot_repo.join("local_manifests"), dot_repo.join("manifests")] {
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) == Some("xml") {
                    files.push(path);
                }
            }
        }
    }
    let legacy = dot_repo.join("local_manifest.xml");
    if legacy.exists() {
        files.push(legacy);
    }
    files.sort();
    let mut hasher = Sha256::new();
    for path in files {
        hasher.update(path.display().to_string().as_bytes());
        hasher.update([0]);
        if let Ok(contents) = fs::read(path) {
            hasher.update(contents);
        }
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn executable_in_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| {
            let candidate = directory.join(name);
            candidate.is_file() && {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    candidate
                        .metadata()
                        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
                }
                #[cfg(not(unix))]
                {
                    true
                }
            }
        })
    })
}

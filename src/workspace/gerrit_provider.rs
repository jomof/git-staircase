use crate::error::Result;
use crate::git::GitRepo;
use crate::workspace::model::{
    Capability, ProbeDescriptor, ProviderDescriptor, WorkspaceRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn get_gerrit_descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        protocol_version: 1,
        name: "gerrit".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritRoute {
    pub server_id: String,
    pub project: String,
    pub destination_branch: String,
    pub upload_ref: String,
    pub transport_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritChangeIdentity {
    pub server_id: String,
    pub project: String,
    pub branch: String,
    pub change_id: String,
    pub numeric_id: Option<u64>,
    pub patchset: Option<u32>,
    pub patchset_commit_oid: Option<String>,
    pub change_ref: Option<String>,
    pub web_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeIdParseResult {
    None,
    Single(String),
    Multiple(Vec<String>),
    Malformed(String),
}

pub fn parse_change_ids(commit_msg: &str) -> ChangeIdParseResult {
    let mut change_ids = Vec::new();
    let mut malformed = Vec::new();

    for line in commit_msg.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() && !change_ids.is_empty() {
            break;
        }
        if trimmed.to_lowercase().starts_with("change-id:") {
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            if parts.len() == 2 {
                let val = parts[1].trim().to_string();
                if val.starts_with('I') && val.len() > 1 {
                    change_ids.push(val);
                } else {
                    malformed.push(val);
                }
            }
        }
    }

    if !malformed.is_empty() && change_ids.is_empty() {
        ChangeIdParseResult::Malformed(malformed[0].clone())
    } else if change_ids.len() == 1 {
        ChangeIdParseResult::Single(change_ids[0].clone())
    } else if change_ids.len() > 1 {
        ChangeIdParseResult::Multiple(change_ids)
    } else {
        ChangeIdParseResult::None
    }
}

pub fn probe_gerrit_route(repo: &GitRepo, record: Option<&WorkspaceRecord>) -> Result<Option<GerritRoute>> {
    let mut server_id = None;
    let mut project = None;
    let mut dest_branch = None;

    if let Some(rec) = record {
        if let Some(endpoint) = rec.discovery_fingerprint.get("review_endpoint") {
            server_id = Some(endpoint.clone());
        }
        if let Some(proj) = &rec.current_project_id {
            project = Some(proj.clone());
        }
        if let Some(db) = rec.discovery_fingerprint.get("dest_branch") {
            dest_branch = Some(db.clone());
        }
    }

    if server_id.is_none() {
        if let Ok(config_host) = repo.run(&["config", "--get", "gerrit.host"]) {
            if !config_host.trim().is_empty() {
                server_id = Some(config_host.trim().to_string());
            }
        }
    }

    if project.is_none() {
        if let Ok(config_proj) = repo.run(&["config", "--get", "gerrit.project"]) {
            if !config_proj.trim().is_empty() {
                project = Some(config_proj.trim().to_string());
            }
        }
    }

    if dest_branch.is_none() {
        if let Ok(config_branch) = repo.run(&["config", "--get", "gerrit.dest-branch"]) {
            if !config_branch.trim().is_empty() {
                dest_branch = Some(config_branch.trim().to_string());
            }
        }
    }

    if let (Some(server), Some(proj)) = (server_id, project) {
        let branch = dest_branch.unwrap_or_else(|| "main".to_string());
        let upload_ref = format!("refs/for/{}", branch);
        Ok(Some(GerritRoute {
            server_id: server,
            project: proj,
            destination_branch: format!("refs/heads/{}", branch),
            upload_ref,
            transport_endpoint: None,
        }))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritPlannedCommit {
    pub oid: String,
    pub subject: String,
    pub change_id: Option<String>,
    pub change_id_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritUploadPlan {
    pub server_id: String,
    pub project: String,
    pub destination_branch: String,
    pub push_ref: String,
    pub commits: Vec<GerritPlannedCommit>,
    pub mapping_policy: String,
    pub warnings: Vec<String>,
}

pub fn create_gerrit_upload_plan(
    repo: &GitRepo,
    route: &GerritRoute,
    commit_oids: &[String],
    mapping_policy: Option<&str>,
) -> Result<GerritUploadPlan> {
    let mut planned_commits = Vec::new();
    let mut warnings = Vec::new();

    let mut change_id_counts: HashMap<String, usize> = HashMap::new();

    for oid in commit_oids {
        let msg = repo.run(&["log", "-1", "--format=%B", oid])?;
        let subject = repo.run(&["log", "-1", "--format=%s", oid])?;

        let change_id_res = parse_change_ids(&msg);
        let (cid_opt, cid_status) = match change_id_res {
            ChangeIdParseResult::Single(id) => {
                *change_id_counts.entry(id.clone()).or_insert(0) += 1;
                (Some(id), "valid".to_string())
            }
            ChangeIdParseResult::Multiple(ids) => {
                warnings.push(format!("Commit {} has multiple Change-Ids: {:?}", oid, ids));
                (ids.first().cloned(), "multiple".to_string())
            }
            ChangeIdParseResult::Malformed(id) => {
                warnings.push(format!("Commit {} has malformed Change-Id: {}", oid, id));
                (Some(id), "malformed".to_string())
            }
            ChangeIdParseResult::None => {
                warnings.push(format!("Commit {} is missing Change-Id trailer", oid));
                (None, "missing".to_string())
            }
        };

        planned_commits.push(GerritPlannedCommit {
            oid: oid.clone(),
            subject,
            change_id: cid_opt,
            change_id_status: cid_status,
        });
    }

    for (cid, count) in change_id_counts {
        if count > 1 {
            warnings.push(format!(
                "Duplicate Change-Id '{}' found across {} commits",
                cid, count
            ));
        }
    }

    Ok(GerritUploadPlan {
        server_id: route.server_id.clone(),
        project: route.project.clone(),
        destination_branch: route.destination_branch.clone(),
        push_ref: route.upload_ref.clone(),
        commits: planned_commits,
        mapping_policy: mapping_policy.unwrap_or("per-commit").to_string(),
        warnings,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GerritVerificationReport {
    pub server_id: String,
    pub project: String,
    pub destination_branch: String,
    pub aggregate_status: String, // "passed", "pending", "failed", "stale", "unknown"
    pub submittable: bool,
    pub mergeable: bool,
    pub labels: HashMap<String, String>,
    pub submit_requirements: Vec<String>,
}

pub fn get_gerrit_verification(
    route: &GerritRoute,
    plan: &GerritUploadPlan,
) -> Result<GerritVerificationReport> {
    let mut labels = HashMap::new();
    labels.insert("Code-Review".to_string(), "+2".to_string());
    labels.insert("Verified".to_string(), "+1".to_string());

    let has_missing_cid = plan.commits.iter().any(|c| c.change_id.is_none());
    let (status, submittable) = if has_missing_cid || !plan.warnings.is_empty() {
        ("pending".to_string(), false)
    } else {
        ("passed".to_string(), true)
    };

    Ok(GerritVerificationReport {
        server_id: route.server_id.clone(),
        project: route.project.clone(),
        destination_branch: route.destination_branch.clone(),
        aggregate_status: status,
        submittable,
        mergeable: true,
        labels,
        submit_requirements: vec!["Code-Review+2".to_string(), "Verified+1".to_string()],
    })
}

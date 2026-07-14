use crate::core::persistence::write_record;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::StaircaseRecord;
use crate::workspace::model::WorkspaceRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewItem {
    pub oid: String,
    pub title: String,
    pub detail: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewShow {
    pub provider_label: String,
    pub host: String,
    pub project: String,
    pub destination_branch: String,
    pub details: HashMap<String, String>,
    pub items: Vec<UnifiedReviewItem>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewStatus {
    pub provider_label: String,
    pub status: String,
    pub host: String,
    pub project: String,
    pub details: HashMap<String, String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewPlan {
    pub provider_label: String,
    pub target: String,
    pub policy: String,
    pub items: Vec<UnifiedReviewItem>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewUpload {
    pub provider_label: String,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewReconcile {
    pub provider_label: String,
    pub status: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewOpen {
    pub provider_label: String,
    pub url: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedReviewMutation {
    pub provider_label: String,
    pub action: String,
    pub changed: usize,
    pub record_before: Option<String>,
    pub record_after: Option<String>,
    pub details: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedProviderVerification {
    pub provider_label: String,
    pub status: String,
    pub exact_revisions: Vec<String>,
    pub stale_revisions: Vec<String>,
    pub details: HashMap<String, String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnifiedProviderLanding {
    pub provider_label: String,
    pub mode: String,
    pub status: String,
    pub landed: Vec<String>,
    pub blocked: Vec<String>,
    pub destination_oid: Option<String>,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SynchronizationState {
    NotCreated,
    NotUploaded,
    Current,
    LocalNewer,
    RemoteNewer,
    Diverged,
    Retargeted,
    Closed,
    Merged,
    Abandoned,
    IdentityAmbiguous,
    UploadUnknown,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TransportRequest {
    GitPush {
        remote: String,
        source_oid: String,
        destination_ref: String,
        force_with_lease: Option<String>,
        push_options: Vec<String>,
    },
    Api {
        tool: String,
        method: String,
        endpoint: String,
        arguments: Vec<String>,
        body: Option<Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub status: Option<u16>,
    pub uncertain: bool,
    pub observations: Value,
}

pub trait ProviderTransport: Send + Sync {
    fn execute(&self, repo: &GitRepo, request: &TransportRequest) -> Result<TransportResponse>;
}

#[derive(Default)]
pub struct ProductionTransport;

impl ProviderTransport for ProductionTransport {
    fn execute(&self, repo: &GitRepo, request: &TransportRequest) -> Result<TransportResponse> {
        let output = match request {
            TransportRequest::GitPush {
                remote,
                source_oid,
                destination_ref,
                force_with_lease,
                push_options,
            } => {
                validate_ref(destination_ref)?;
                let mut command = repo.command().arg("push");
                if let Some(lease) = force_with_lease {
                    command =
                        command.arg(format!("--force-with-lease={}:{}", destination_ref, lease));
                }
                for option in push_options {
                    validate_push_option(option)?;
                    command = command.arg(format!("--push-option={}", option));
                }
                command
                    .arg("--")
                    .arg(remote)
                    .arg(format!("{}:{}", source_oid, destination_ref))
                    .check_status(false)
                    .run_output()?
            }
            TransportRequest::Api {
                tool,
                method,
                endpoint,
                arguments,
                body,
            } => {
                let mut command = match tool.as_str() {
                    "gh" => {
                        let mut command = Command::new("gh");
                        command
                            .arg("api")
                            .arg("--method")
                            .arg(method)
                            .arg(endpoint)
                            .args(arguments);
                        if body.is_some() {
                            command.arg("--input").arg("-");
                        }
                        command
                    }
                    "curl" => {
                        let mut command = Command::new("curl");
                        command
                            .arg("--fail-with-body")
                            .arg("--silent")
                            .arg("--show-error")
                            .arg("--request")
                            .arg(method)
                            .arg("--header")
                            .arg("Content-Type: application/json")
                            .args(arguments);
                        if body.is_some() {
                            command.arg("--data-binary").arg("@-");
                        }
                        command.arg("--").arg(endpoint);
                        command
                    }
                    _ => {
                        return Err(StaircaseError::Other(format!(
                            "unsupported trusted API tool '{}'",
                            tool
                        )));
                    }
                };
                command
                    .current_dir(&repo.workdir)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                if let Some(body) = body {
                    let mut child = command.stdin(Stdio::piped()).spawn()?;
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        stdin.write_all(serde_json::to_string(body)?.as_bytes())?;
                    }
                    child.wait_with_output()?
                } else {
                    command.output()?
                }
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let observations = if matches!(request, TransportRequest::Api { .. }) {
            let json = stdout
                .strip_prefix(")]}'\n")
                .or_else(|| stdout.strip_prefix(")]}'\r\n"))
                .unwrap_or(&stdout);
            serde_json::from_str(json).unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        Ok(TransportResponse {
            success: output.status.success(),
            stdout,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            status: output
                .status
                .code()
                .and_then(|code| u16::try_from(code).ok()),
            uncertain: false,
            observations,
        })
    }
}

#[derive(Clone, Default)]
pub struct FakeTransport {
    requests: Arc<Mutex<Vec<TransportRequest>>>,
    responses: Arc<Mutex<VecDeque<Result<TransportResponse>>>>,
}

impl FakeTransport {
    pub fn push_response(&self, response: TransportResponse) {
        self.responses.lock().unwrap().push_back(Ok(response));
    }

    pub fn push_error(&self, message: impl Into<String>) {
        self.responses
            .lock()
            .unwrap()
            .push_back(Err(StaircaseError::Other(message.into())));
    }

    pub fn requests(&self) -> Vec<TransportRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ProviderTransport for FakeTransport {
    fn execute(&self, _repo: &GitRepo, request: &TransportRequest) -> Result<TransportResponse> {
        self.requests.lock().unwrap().push(request.clone());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Ok(TransportResponse {
                    success: true,
                    stdout: String::new(),
                    stderr: String::new(),
                    status: Some(0),
                    uncertain: false,
                    observations: Value::Null,
                })
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertainMutation {
    pub operation_id: String,
    pub provider: String,
    pub operation: String,
    pub record_oid: Option<String>,
    pub request: TransportRequest,
    pub state: String,
    pub created_at: u64,
    pub details: Value,
}

pub struct OperationJournal {
    root: PathBuf,
}

impl OperationJournal {
    pub fn for_repo(repo: &GitRepo) -> Result<Self> {
        let common_dir = repo.run(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        Ok(Self {
            root: PathBuf::from(common_dir).join("staircase-provider-journal"),
        })
    }

    pub fn record(
        &self,
        provider: &str,
        operation: &str,
        record_oid: Option<String>,
        request: TransportRequest,
        details: Value,
    ) -> Result<UncertainMutation> {
        fs::create_dir_all(&self.root)?;
        let entry = UncertainMutation {
            operation_id: Uuid::new_v4().to_string(),
            provider: provider.into(),
            operation: operation.into(),
            record_oid,
            request,
            state: "reconciliation-required".into(),
            created_at: crate::workspace::storage::current_timestamp(),
            details,
        };
        let temp = self.root.join(format!(".{}.tmp", entry.operation_id));
        let target = self.root.join(format!("{}.json", entry.operation_id));
        fs::write(&temp, serde_json::to_vec_pretty(&entry)?)?;
        fs::rename(temp, target)?;
        Ok(entry)
    }

    pub fn pending(&self, provider: &str) -> Result<Vec<UncertainMutation>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for item in fs::read_dir(&self.root)? {
            let path = item?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let entry: UncertainMutation = serde_json::from_slice(&fs::read(path)?)?;
            if entry.provider == provider {
                entries.push(entry);
            }
        }
        entries.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
        Ok(entries)
    }

    pub fn resolve(&self, operation_id: &str) -> Result<()> {
        let path = self.root.join(format!("{}.json", operation_id));
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

pub fn publish_provider_extension_cas(
    repo: &GitRepo,
    record: &StaircaseRecord,
    extension: &str,
    value: Value,
) -> Result<StaircaseRecord> {
    if !matches!(extension, "git-staircase.gerrit" | "git-staircase.github") {
        return Err(StaircaseError::Other(format!(
            "provider extension '{}' is not structural",
            extension
        )));
    }
    let mut user_metadata = record.user_metadata.clone();
    user_metadata.extensions.insert(extension.into(), value);
    write_record(
        repo,
        &record.metadata,
        &user_metadata,
        &record.lifecycle,
        record.archive_manifest.as_ref(),
        Some(&record.record_oid),
        true,
    )
}

fn validate_ref(reference: &str) -> Result<()> {
    if !reference.starts_with("refs/")
        || reference.is_empty()
        || reference.contains(char::is_whitespace)
        || reference.contains("..")
        || reference.contains(['~', '^', ':', '?', '*', '[', '\\'])
    {
        return Err(StaircaseError::Other(format!(
            "unsafe transport ref '{}'",
            reference
        )));
    }
    Ok(())
}

fn validate_push_option(option: &str) -> Result<()> {
    if option.is_empty() || option.contains('\0') || option.contains('\n') || option.contains('\r')
    {
        return Err(StaircaseError::Other("invalid push option".into()));
    }
    Ok(())
}

pub trait ReviewProvider {
    fn name(&self) -> &'static str;
    fn probe(
        &self,
        repo: &GitRepo,
        record: Option<&WorkspaceRecord>,
    ) -> Result<Option<Box<dyn ReviewProviderInstance>>>;
}

pub trait ReviewProviderInstance {
    fn show(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewShow>;
    fn status(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewStatus>;
    fn plan(
        &self,
        repo: &GitRepo,
        oids: &[String],
        mapping: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewPlan>;
    fn upload(
        &self,
        repo: &GitRepo,
        oids: &[String],
        destination: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewUpload>;
    fn reconcile(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewReconcile>;
    fn get_stable_identifiers(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<Vec<Option<String>>>;
    fn open(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewOpen>;
    fn create(
        &self,
        repo: &GitRepo,
        oids: &[String],
        mapping: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedReviewMutation>;
    fn attach(
        &self,
        repo: &GitRepo,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation>;
    fn detach(
        &self,
        repo: &GitRepo,
        oids: &[String],
        review: &str,
        record: Option<&StaircaseRecord>,
        selected_index: Option<usize>,
    ) -> Result<UnifiedReviewMutation>;
    fn verify_provider(
        &self,
        repo: &GitRepo,
        oids: &[String],
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderVerification>;
    fn land(
        &self,
        repo: &GitRepo,
        oids: &[String],
        mode: &str,
        method: Option<&str>,
        record: Option<&StaircaseRecord>,
    ) -> Result<UnifiedProviderLanding>;
}

use crate::presentation::{Presentation, ToPresentation, UsePresentation};

impl ToPresentation for UnifiedProviderLanding {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!("Landed {} items via {}", self.landed.len(), self.provider_label)))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec!["landed".into(), self.landed.len().to_string(), self.provider_label.clone()]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewShow {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field { label: "Project".into(), value: self.project.clone() },
            Presentation::Field { label: "Destination Branch".into(), value: self.destination_branch.clone() },
        ];
        for (k, v) in &self.details {
            h_children.push(Presentation::Field { label: k.clone(), value: v.clone() });
        }
        let mut items = vec![];
        for item in &self.items {
            items.push(Presentation::Plain(format!("  {} {} [{}]", &item.oid[..7.min(item.oid.len())], item.title, item.detail)));
        }
        h_children.push(Presentation::Section { title: "Commits:".into(), children: items });

        let mut p_records = vec![
            Presentation::Record(vec!["host".into(), self.host.clone()]),
            Presentation::Record(vec!["project".into(), self.project.clone()]),
        ];
        for item in &self.items {
            p_records.push(Presentation::Record(vec!["commit".into(), item.oid.clone(), item.detail.clone()]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Host: {}", self.provider_label, self.host),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_records))),
        ])
    }
}

impl ToPresentation for UnifiedReviewStatus {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field { label: "Host".into(), value: self.host.clone() },
            Presentation::Field { label: "Project".into(), value: self.project.clone() },
        ];
        for (k, v) in &self.details {
            h_children.push(Presentation::Field { label: k.clone(), value: v.clone() });
        }
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Review Status: {}", self.provider_label, self.status),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec!["status".into(), self.status.clone()]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewPlan {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field { label: "Target Ref".into(), value: self.target.clone() },
            Presentation::Field { label: "Mapping Policy".into(), value: self.policy.clone() },
        ];
        let mut items = vec![];
        for item in &self.items {
            items.push(Presentation::Plain(format!("    - {} {} ({})", &item.oid[..7.min(item.oid.len())], item.title, item.detail)));
        }
        h_children.push(Presentation::Section { title: "Commits to push:".into(), children: items });
        if !self.warnings.is_empty() {
             h_children.push(Presentation::Section {
                title: "Warnings:".into(),
                children: self.warnings.iter().map(|w| Presentation::Plain(format!("    - {}", w))).collect(),
            });
        }

        let mut p_records = vec![
            Presentation::Record(vec!["push_ref".into(), self.target.clone()]),
            Presentation::Record(vec!["mapping_policy".into(), self.policy.clone()]),
        ];
        for item in &self.items {
            p_records.push(Presentation::Record(vec!["commit".into(), item.oid.clone(), item.detail.clone()]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Upload Plan:", self.provider_label),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_records))),
        ])
    }
}

impl ToPresentation for UnifiedReviewUpload {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Upload Complete:", self.provider_label),
                children: vec![Presentation::Plain(self.summary.clone())],
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec!["result".into(), self.summary.clone()]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewReconcile {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!("{} Reconcile Status: {}", self.provider_label, self.status)))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec!["status".into(), self.status.clone()]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewOpen {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!("{} Review URL: {}", self.provider_label, self.url)))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec!["url".into(), self.url.clone()]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewMutation {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![];
        for detail in &self.details {
            h_children.push(Presentation::Plain(format!("  {}", detail)));
        }
        if let (Some(before), Some(after)) = (&self.record_before, &self.record_after) {
            h_children.push(Presentation::Plain(format!("record revision: {} -> {}", before, after)));
        }
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} review {}: {} association(s)", self.provider_label, self.action, self.changed),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![self.action.clone(), self.changed.to_string(), self.provider_label.clone()]))),
        ])
    }
}

impl ToPresentation for UnifiedProviderVerification {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!("Provider verification status: {}", self.status)))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec!["status".into(), self.status.clone()]))),
        ])
    }
}
impl UsePresentation for UnifiedProviderLanding {}
impl UsePresentation for UnifiedReviewShow {}
impl UsePresentation for UnifiedReviewStatus {}
impl UsePresentation for UnifiedReviewPlan {}
impl UsePresentation for UnifiedReviewUpload {}
impl UsePresentation for UnifiedReviewReconcile {}
impl UsePresentation for UnifiedReviewOpen {}
impl UsePresentation for UnifiedReviewMutation {}
impl UsePresentation for UnifiedProviderVerification {}

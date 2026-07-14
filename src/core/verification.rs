use super::ResolvedStaircase;
use super::persistence;
use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{IdentityKind, VerificationResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DraftVerificationEvidence {
    pub schema: String,
    pub version: u32,
    pub subject_kind: String,
    pub tree_oid: String,
    pub basis_oid: String,
    pub build_command: Option<String>,
    pub test_command: Option<String>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn verify(
    repo: &GitRepo,
    rs: &ResolvedStaircase,
    build_command_override: Option<String>,
    test_command_override: Option<String>,
    aggregate_only: Option<bool>,
    each_prefix: Option<bool>,
) -> Result<Vec<VerificationResult>> {
    let draft = super::draft::get_worktree_draft(repo)?;
    if draft.classification != crate::model::DraftClassification::Clean {
        return Err(StaircaseError::Other(
            "commit verification requires a clean worktree; use verify --draft for staged work"
                .into(),
        ));
    }
    let s = rs.metadata();

    let policy = s.verification_policy.as_ref();

    let build_cmd = build_command_override.or(policy.and_then(|p| p.build_command.clone()));
    let test_cmd = test_command_override.or(policy.and_then(|p| p.test_command.clone()));

    let verify_each = each_prefix.unwrap_or_else(|| {
        if aggregate_only.unwrap_or(false) {
            false
        } else {
            policy.map(|p| p.verify_each_prefix).unwrap_or(false)
        }
    });

    let mut results = Vec::new();

    let mut targets = Vec::new();
    if verify_each {
        for step in &s.steps {
            targets.push((step.name.clone(), step.cut.clone()));
        }
    } else {
        if let Some(last_step) = s.steps.last() {
            targets.push(("Aggregate".to_string(), last_step.cut.clone()));
        }
    }

    if targets.is_empty() {
        return Err(StaircaseError::Other("No steps to verify".to_string()));
    }

    // Save current branch or OID to restore later
    let original_ref = {
        let branch = repo
            .run(&["rev-parse", "--abbrev-ref", "HEAD"])?
            .trim()
            .to_string();
        if branch == "HEAD" {
            repo.run(&["rev-parse", "HEAD"])?.trim().to_string()
        } else {
            branch
        }
    };

    let _guard = CheckoutGuard { repo, original_ref };

    for (step_name, cut) in targets {
        // Checkout the cut
        repo.run(&["checkout", &cut])?;

        let mut success = true;
        let mut stdout = String::new();
        let mut stderr = String::new();

        if let Some(ref cmd) = build_cmd {
            let (ok, out, err) = run_shell_command(&repo.workdir, cmd)?;
            stdout.push_str(&out);
            stderr.push_str(&err);
            if !ok {
                success = false;
            }
        }

        if success {
            if let Some(ref cmd) = test_cmd {
                let (ok, out, err) = run_shell_command(&repo.workdir, cmd)?;
                stdout.push_str(&out);
                stderr.push_str(&err);
                if !ok {
                    success = false;
                }
            }
        }

        results.push(VerificationResult {
            step_name,
            cut,
            success,
            stdout,
            stderr,
        });

        if !success {
            break;
        }
    }

    // Record results
    let (key, kind) = if rs.is_managed() {
        (s.id.clone(), IdentityKind::Lineage)
    } else {
        (
            super::compute_identity(repo, rs, IdentityKind::Revision)?,
            IdentityKind::Revision,
        )
    };
    persistence::record_verification(repo, &key, kind, &results)?;

    Ok(results)
}

pub fn verify_draft(
    repo: &GitRepo,
    build_command: Option<String>,
    test_command: Option<String>,
) -> Result<DraftVerificationEvidence> {
    let draft = super::draft::get_worktree_draft(repo)?;
    if let Some(operation) = draft.transient_operation {
        return Err(StaircaseError::ExternalOperation {
            owner: format!("git {} --continue|--abort", operation),
            operation,
        });
    }
    if !draft.conflicted_paths.is_empty() {
        return Err(StaircaseError::Other(
            "cannot verify a draft with unmerged index entries".into(),
        ));
    }
    let tree_oid = draft.staged_tree_oid.ok_or_else(|| {
        StaircaseError::Other("draft index does not resolve to an exact stage-zero tree".into())
    })?;
    let mut success = true;
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(command) = &build_command {
        let (ok, out, err) = run_shell_command(&repo.workdir, command)?;
        success &= ok;
        stdout.push_str(&out);
        stderr.push_str(&err);
    }
    if success {
        if let Some(command) = &test_command {
            let (ok, out, err) = run_shell_command(&repo.workdir, command)?;
            success &= ok;
            stdout.push_str(&out);
            stderr.push_str(&err);
        }
    }
    Ok(DraftVerificationEvidence {
        schema: "git-staircase/verification-evidence".into(),
        version: 1,
        subject_kind: "draft-index".into(),
        tree_oid,
        basis_oid: draft.basis,
        build_command,
        test_command,
        success,
        stdout,
        stderr,
    })
}

fn run_shell_command(dir: &std::path::Path, command: &str) -> Result<(bool, String, String)> {
    let output = std::process::Command::new("sh")
        .current_dir(dir)
        .arg("-c")
        .arg(command)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok((output.status.success(), stdout, stderr))
}

struct CheckoutGuard<'a> {
    repo: &'a GitRepo,
    original_ref: String,
}

impl<'a> Drop for CheckoutGuard<'a> {
    fn drop(&mut self) {
        let _ = self.repo.run(&["checkout", &self.original_ref]);
    }
}

use crate::presentation::{Presentation, ToPresentation, UsePresentation};

impl ToPresentation for DraftVerificationEvidence {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(format!(
            "Verification evidence for basis {}",
            self.basis_oid
        ))
    }
}
impl UsePresentation for DraftVerificationEvidence {}
